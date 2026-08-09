//! Tauri 命令：前后端桥梁。

use crate::classfile::{self, ConstantTag};
use crate::decompiler;
use crate::jar;
use crate::jdk;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::AppHandle;

/// class 字节来源：独立文件或 JAR 内条目。
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ClassSource {
    File { path: String },
    Jar { jar_path: String, entry_name: String },
}

impl ClassSource {
    fn read_bytes(&self) -> Result<Vec<u8>, String> {
        match self {
            ClassSource::File { path } => std::fs::read(path).map_err(|e| e.to_string()),
            ClassSource::Jar { jar_path, entry_name } => {
                jar::read_entry(jar_path, entry_name).map_err(|e| e.to_string())
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StringEntry {
    pub index: u16,
    pub value: String,
    pub is_literal: bool,
    pub byte_length: usize,
}

#[derive(Debug, Serialize)]
pub struct ClassFilePreview {
    pub class_name: String,
    pub version: String,
    pub strings: Vec<StringEntry>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum FilePreview {
    Class { path: String, preview: ClassFilePreview },
    Jar { info: jar::JarInfo },
}

#[derive(Debug, Deserialize)]
pub struct Modification {
    pub index: u16,
    pub new_value: String,
}

/// 统一打开入口：根据扩展名分发。
#[tauri::command]
pub fn open_file(path: String) -> Result<FilePreview, String> {
    let lower = path.to_lowercase();
    if lower.ends_with(".class") {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        let preview = parse_preview(&bytes)?;
        Ok(FilePreview::Class { path, preview })
    } else if lower.ends_with(".jar") {
        let info = jar::read_jar_info(&path).map_err(|e| e.to_string())?;
        Ok(FilePreview::Jar { info })
    } else {
        Err("不支持的文件类型（仅支持 .class / .jar）".into())
    }
}

/// 从 JAR 中读取指定 class 并解析。
#[tauri::command]
pub fn open_class_in_jar(jar_path: String, entry_name: String) -> Result<ClassFilePreview, String> {
    let bytes = jar::read_entry(&jar_path, &entry_name).map_err(|e| e.to_string())?;
    parse_preview(&bytes)
}

/// 保存修改到独立 .class 文件。
#[tauri::command]
pub fn save_class_file(path: String, modifications: Vec<Modification>) -> Result<(), String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let mut cf = classfile::parse_classfile(&bytes).map_err(|e| e.to_string())?;
    apply_modifications(&mut cf, &modifications);
    let out = classfile::serialize_classfile(&cf);
    // 备份
    let backup = format!("{}.bak", path);
    let _ = std::fs::copy(&path, &backup);
    std::fs::write(&path, out).map_err(|e| e.to_string())
}

/// 保存修改到 JAR 内的 class 条目（单条目替换）。
#[tauri::command]
pub fn save_class_in_jar(
    jar_path: String,
    entry_name: String,
    modifications: Vec<Modification>,
) -> Result<(), String> {
    let bytes = jar::read_entry(&jar_path, &entry_name).map_err(|e| e.to_string())?;
    let mut cf = classfile::parse_classfile(&bytes).map_err(|e| e.to_string())?;
    apply_modifications(&mut cf, &modifications);
    let out = classfile::serialize_classfile(&cf);
    jar::replace_entry(&jar_path, &entry_name, &out).map_err(|e| e.to_string())
}

/// 检测 JDK。
#[tauri::command]
pub fn detect_jdk() -> Result<Option<jdk::JdkInfo>, String> {
    Ok(jdk::detect_jdk())
}

/// 设置并验证自定义 JDK 路径。
#[tauri::command]
pub fn set_jdk_path(app: AppHandle, path: String) -> Result<jdk::JdkInfo, String> {
    let info = jdk::validate_jdk(&path).ok_or("无效的 JDK 路径".to_string())?;
    jdk::save_config(&app, &path).map_err(|e| e)?;
    Ok(info)
}

/// 反编译 class。优先使用配置/检测到的 JDK。
#[tauri::command]
pub fn decompile_class(app: AppHandle, source: ClassSource) -> Result<decompiler::DecompileResult, String> {
    let bytes = source.read_bytes()?;
    let jdk_path = resolve_jdk(&app)?;
    decompiler::decompile(&app, &bytes, &jdk_path).map_err(|e| e.to_string())
}

/// 获取字节码（javap）。
#[tauri::command]
pub fn get_bytecode(app: AppHandle, source: ClassSource) -> Result<String, String> {
    let bytes = source.read_bytes()?;
    let jdk_path = resolve_jdk(&app)?;
    decompiler::get_bytecode(&bytes, &jdk_path).map_err(|e| e.to_string())
}

// ---- 内部辅助 ----

fn resolve_jdk(app: &AppHandle) -> Result<String, String> {
    // 优先用户配置
    if let Some(custom) = jdk::load_config(app) {
        if jdk::validate_jdk(&custom).is_some() {
            return Ok(custom);
        }
    }
    // 回退自动检测
    jdk::detect_jdk()
        .map(|i| i.path)
        .ok_or_else(|| "未检测到 JDK，请在设置中配置".to_string())
}

fn parse_preview(bytes: &[u8]) -> Result<ClassFilePreview, String> {
    let cf = classfile::parse_classfile(bytes).map_err(|e| e.to_string())?;
    let literals: HashSet<u16> = cf.constant_pool.literal_utf8_indices();
    let mut strings = Vec::new();
    for (idx, value) in cf.constant_pool.utf8_entries() {
        strings.push(StringEntry {
            index: idx,
            value: value.to_string(),
            is_literal: literals.contains(&idx),
            byte_length: value.len(),
        });
    }
    // 类名
    let class_name = cf
        .constant_pool
        .get(extract_this_class_index(&cf))
        .and_then(|e| match e {
            classfile::ConstantEntry::Other { tag: ConstantTag::Class, bytes } => {
                if bytes.len() >= 2 {
                    let name_idx = u16::from_be_bytes([bytes[0], bytes[1]]);
                    cf.constant_pool.get(name_idx).and_then(|ne| ne.as_utf8_value()).map(String::from)
                } else {
                    None
                }
            }
            _ => None,
        })
        .unwrap_or_default();
    Ok(ClassFilePreview {
        class_name,
        version: format!("{}.{}", cf.major_version, cf.minor_version),
        strings,
    })
}

/// 从 tail_bytes 开头读取 this_class 索引（access_flags 之后第一个 u2）。
fn extract_this_class_index(cf: &classfile::ClassFile) -> u16 {
    // tail_bytes 顺序: access_flags(2) this_class(2) super_class(2) ...
    if cf.tail_bytes.len() >= 4 {
        u16::from_be_bytes([cf.tail_bytes[2], cf.tail_bytes[3]])
    } else {
        0
    }
}

fn apply_modifications(cf: &mut classfile::ClassFile, modifications: &[Modification]) {
    for m in modifications {
        if let Some(entry) = cf.constant_pool.get_mut(m.index) {
            entry.set_utf8_value(m.new_value.clone());
        }
    }
}
