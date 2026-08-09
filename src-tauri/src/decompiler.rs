//! CFR 反编译器集成 + javap 字节码 + 字符串位置定位。
//!
//! CFR JAR 通过 `include_bytes!` 直接嵌入到二进制中，
//! 首次使用时释放到系统临时目录，保证单 exe 完全自包含。

use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tauri::AppHandle;
use thiserror::Error;

/// 编译期嵌入 CFR 反编译器 JAR（相对路径基于 lib.rs 所在目录）。
const CFR_JAR_BYTES: &[u8] = include_bytes!("../resources/cfr-0.152.jar");

#[derive(Debug, Error)]
pub enum DecompileError {
    #[error("未检测到 JDK，请在设置中配置 JDK 路径")]
    NoJdk,
    #[error("CFR 反编译器 JAR 缺失: {0}")]
    CfrMissing(String),
    #[error("反编译失败: {0}")]
    Failed(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct DecompileResult {
    pub source: String,
    pub decompiler_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextRange {
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

/// 反编译 class 字节为 Java 源码。
pub fn decompile(
    _app: &AppHandle,
    class_bytes: &[u8],
    jdk_path: &str,
) -> Result<DecompileResult, DecompileError> {
    let cfr_jar = ensure_cfr_jar()?;
    let java_bin = java_executable(jdk_path)?;

    // 写入临时 class 文件
    let temp_dir = std::env::temp_dir();
    let class_stem = format!("editclass_{}", random_id());
    let temp_class = temp_dir.join(format!("{}.class", class_stem));
    {
        let mut f = fs::File::create(&temp_class)?;
        f.write_all(class_bytes)?;
    }

    let output = Command::new(&java_bin)
        .arg("-Dfile.encoding=UTF-8")
        .arg("-jar")
        .arg(&cfr_jar)
        .arg(&temp_class)
        .arg("--comments")
        .arg("false")
        .output()
        .map_err(|e| DecompileError::Failed(e.to_string()))?;

    let _ = fs::remove_file(&temp_class);

    if !output.status.success() {
        let err = decode_text(&output.stderr);
        // 失败可能同时有部分 stdout，也显示出来
        let out = decode_text(&output.stdout);
        let msg = if !err.is_empty() { err } else { out };
        return Err(DecompileError::Failed(msg));
    }
    let source = decode_text(&output.stdout);

    Ok(DecompileResult {
        source,
        decompiler_version: "CFR 0.152".to_string(),
    })
}

/// 获取字节码（javap -c -p）。
pub fn get_bytecode(class_bytes: &[u8], jdk_path: &str) -> Result<String, DecompileError> {
    let javap_bin = javap_executable(jdk_path)?;

    let temp_dir = std::env::temp_dir();
    let class_stem = format!("editclass_{}", random_id());
    let temp_class = temp_dir.join(format!("{}.class", class_stem));
    {
        let mut f = fs::File::create(&temp_class)?;
        f.write_all(class_bytes)?;
    }

    let output = Command::new(&javap_bin)
        .arg("-J-Dfile.encoding=UTF-8")
        .arg("-c")
        .arg("-p")
        .arg("-cp")
        .arg(&temp_dir)
        .arg(&class_stem)
        .output()
        .map_err(|e| DecompileError::Failed(e.to_string()))?;

    let _ = fs::remove_file(&temp_class);

    if !output.status.success() {
        let err = decode_text(&output.stderr);
        return Err(DecompileError::Failed(err));
    }
    Ok(decode_text(&output.stdout))
}

/// 在源码中定位目标字符串的所有出现位置（行号 + 列范围）。
pub fn locate_string_occurrences(source: &str, target: &str) -> Vec<TextRange> {
    if target.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let mut start = 0;
        while let Some(pos) = line[start..].find(target) {
            let abs = start + pos;
            ranges.push(TextRange {
                line: i + 1,
                start_col: abs,
                end_col: abs + target.chars().count(),
            });
            start = abs + target.len();
        }
    }
    ranges
}

/// 将嵌入的 CFR JAR 释放到临时目录，返回 JAR 路径。
/// 使用带内容哈希的目录名，多版本共存且避免重复写入。
fn ensure_cfr_jar() -> Result<PathBuf, DecompileError> {
    // 用简单前缀 + 长度标识，避免每次都计算 hash（2MB 成本可接受，但固定更省事）
    let dir_name = format!("editclass_cfr_0.152_{}", CFR_JAR_BYTES.len());
    let target_dir = std::env::temp_dir().join(dir_name);
    let target_jar = target_dir.join("cfr-0.152.jar");
    let exists_and_matches = target_jar.exists()
        && target_jar
            .metadata()
            .map(|m| m.len() as usize == CFR_JAR_BYTES.len())
            .unwrap_or(false);
    if exists_and_matches {
        return Ok(target_jar);
    }
    fs::create_dir_all(&target_dir)?;
    {
        let mut f = fs::File::create(&target_jar)?;
        f.write_all(CFR_JAR_BYTES)?;
    }
    Ok(target_jar)
}

fn java_executable(jdk_path: &str) -> Result<PathBuf, DecompileError> {
    let mut p = PathBuf::from(jdk_path).join("bin").join("java");
    if cfg!(windows) {
        p = p.with_extension("exe");
    }
    if !p.exists() {
        return Err(DecompileError::NoJdk);
    }
    Ok(p)
}

fn javap_executable(jdk_path: &str) -> Result<PathBuf, DecompileError> {
    let mut p = PathBuf::from(jdk_path).join("bin").join("javap");
    if cfg!(windows) {
        p = p.with_extension("exe");
    }
    if !p.exists() {
        return Err(DecompileError::NoJdk);
    }
    Ok(p)
}

fn random_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

/// Windows 上子进程输出默认是 GBK 编码，转 UTF-8 避免乱码。
fn decode_text(bytes: &[u8]) -> String {
    let (cow, _, had_errors) = encoding_rs::GBK.decode(bytes);
    if had_errors {
        // GBK 解码有误差，退回 UTF-8（替换非法字符）
        String::from_utf8_lossy(bytes).to_string()
    } else {
        cow.to_string()
    }
}
