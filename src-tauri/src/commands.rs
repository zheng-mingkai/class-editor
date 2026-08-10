//! Tauri 命令：前后端桥梁。

use crate::classfile::{self, ConstantTag};
use crate::decompiler;
use crate::jar;
use crate::jdk;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
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

#[derive(Debug, Clone, Deserialize)]
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

/// 用 javap -c -l 解析常量池索引 → 源码行号数组映射。
#[tauri::command]
pub fn locate_string_lines(app: AppHandle, source: ClassSource) -> Result<std::collections::HashMap<u16, Vec<usize>>, String> {
    let bytes = source.read_bytes()?;
    let jdk_path = resolve_jdk(&app)?;
    decompiler::locate_constant_indices(&bytes, &jdk_path).map_err(|e| e.to_string())
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

// ==================== 全局搜索替换 ====================

/// 搜索命中结果。
#[derive(Debug, Serialize)]
pub struct SearchHit {
    /// 来源标识：独立文件路径或 JAR 内条目名
    pub source_label: String,
    /// JAR 路径（独立文件时为 None）
    pub jar_path: Option<String>,
    /// JAR 内条目名（独立文件时为 None）
    pub entry_name: Option<String>,
    /// 类名
    pub class_name: String,
    /// 常量池索引
    pub index: u16,
    /// 字符串原值
    pub value: String,
    /// 字节长度
    pub byte_length: usize,
    /// 匹配的预览片段
    pub match_preview: String,
}

/// 搜索范围。
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SearchScope {
    File { path: String },
    Jar { jar_path: String },
}

/// 全局搜索字符串字面量。
/// 在单个 .class 文件或 JAR 内所有 .class 条目中搜索包含 query 的字面量。
#[tauri::command]
pub fn search_strings(scope: SearchScope, query: String) -> Result<Vec<SearchHit>, String> {
    let query_lower = query.to_lowercase();
    let mut hits = Vec::new();

    match scope {
        SearchScope::File { path } => {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            search_in_class_bytes(&bytes, &path, None, None, &query_lower, &query, &mut hits)
                .map_err(|e| e.to_string())?;
        }
        SearchScope::Jar { jar_path } => {
            let entries = jar::list_jar_entries(&jar_path).map_err(|e| e.to_string())?;
            for entry in &entries {
                if entry.is_dir || !entry.name.ends_with(".class") {
                    continue;
                }
                let bytes = jar::read_entry(&jar_path, &entry.name).map_err(|e| e.to_string())?;
                let _ = search_in_class_bytes(
                    &bytes,
                    &entry.name,
                    Some(&jar_path),
                    Some(&entry.name),
                    &query_lower,
                    &query,
                    &mut hits,
                );
            }
        }
    }
    Ok(hits)
}

/// 在单个 class 的字节中搜索字面量。
fn search_in_class_bytes(
    bytes: &[u8],
    source_label: &str,
    jar_path: Option<&str>,
    entry_name: Option<&str>,
    query_lower: &str,
    query: &str,
    hits: &mut Vec<SearchHit>,
) -> Result<(), String> {
    let cf = classfile::parse_classfile(bytes).map_err(|e| e.to_string())?;
    let literals: HashSet<u16> = cf.constant_pool.literal_utf8_indices();
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

    for (idx, value) in cf.constant_pool.utf8_entries() {
        if !literals.contains(&idx) {
            continue;
        }
        let value_lower = value.to_lowercase();
        if value_lower.contains(query_lower) {
            let preview = make_match_preview(value, query);
            hits.push(SearchHit {
                source_label: source_label.to_string(),
                jar_path: jar_path.map(|s| s.to_string()),
                entry_name: entry_name.map(|s| s.to_string()),
                class_name: class_name.clone(),
                index: idx,
                value: value.to_string(),
                byte_length: value.len(),
                match_preview: preview,
            });
        }
    }
    Ok(())
}

/// 生成匹配预览片段（高亮匹配位置附近的内容）。
fn make_match_preview(value: &str, query: &str) -> String {
    let lower = value.to_lowercase();
    let pos = lower.find(&query.to_lowercase());
    match pos {
        Some(pos) => {
            let start = pos.saturating_sub(20);
            let end = (pos + query.len() + 20).min(value.len());
            let mut preview = String::new();
            if start > 0 {
                preview.push_str("…");
            }
            preview.push_str(&value[start..end]);
            if end < value.len() {
                preview.push_str("…");
            }
            preview
        }
        None => value.chars().take(50).collect(),
    }
}

/// 批量替换请求（单条目）。
#[derive(Debug, Deserialize)]
pub struct BatchReplacement {
    pub entry_name: Option<String>,
    pub modifications: Vec<Modification>,
}

/// 批量保存：在 JAR 中同时替换多个 class 条目，或保存独立文件的修改。
/// 对于 JAR：遍历所有条目，对有修改的条目进行替换，无修改的条目原样保留。
#[tauri::command]
pub fn batch_save(
    path: String,
    is_jar: bool,
    replacements: Vec<BatchReplacement>,
) -> Result<usize, String> {
    if is_jar {
        // JAR 模式：收集所有条目的修改
        let mut mods_by_entry: HashMap<String, Vec<Modification>> = HashMap::new();
        for r in &replacements {
            if let Some(name) = &r.entry_name {
                mods_by_entry.insert(name.clone(), r.modifications.clone());
            }
        }
        if mods_by_entry.is_empty() {
            return Ok(0);
        }

        // 创建备份
        let backup = format!("{}.bak", path);
        let _ = std::fs::copy(&path, &backup);

        // 创建临时文件
        let tmp_path = format!("{}.tmp", path);
        {
            let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            let tmp_file = std::fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
            let mut writer = zip::ZipWriter::new(tmp_file);
            let opts: zip::write::SimpleFileOptions =
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);

            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
                let name = entry.name().to_string();
                let is_dir = entry.is_dir();

                if let Some(mods) = mods_by_entry.get(&name) {
                    // 有修改的 class 条目：读取 → 解析 → 修改 → 序列化 → 写入
                    let mut buf = Vec::with_capacity(entry.size() as usize);
                    entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                    let mut cf = classfile::parse_classfile(&buf).map_err(|e| e.to_string())?;
                    apply_modifications(&mut cf, mods);
                    let out = classfile::serialize_classfile(&cf);
                    writer.start_file(&name, opts).map_err(|e| e.to_string())?;
                    writer.write_all(&out).map_err(|e| e.to_string())?;
                } else if is_dir {
                    writer.add_directory(&name, opts).map_err(|e| e.to_string())?;
                } else {
                    // 无修改的条目原样复制
                    writer.start_file(&name, opts).map_err(|e| e.to_string())?;
                    let mut buf = Vec::with_capacity(entry.size() as usize);
                    entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                    writer.write_all(&buf).map_err(|e| e.to_string())?;
                }
            }
            writer.finish().map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
        Ok(mods_by_entry.len())
    } else {
        // 独立文件模式
        let all_mods: Vec<Modification> = replacements
            .into_iter()
            .flat_map(|r| r.modifications)
            .collect();
        if all_mods.is_empty() {
            return Ok(0);
        }
        save_class_file(path, all_mods)?;
        Ok(1)
    }
}

/// 用系统默认浏览器打开 URL。
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    webbrowser::open(&url).map_err(|e| e.to_string())
}
