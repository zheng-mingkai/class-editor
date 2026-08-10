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

    let output = new_command(&java_bin)
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

    let output = new_command(&javap_bin)
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

/// 用 javap -c -l 解析字节码，建立常量池索引 → 源码行号数组映射。
///
/// 同一个常量池索引可能被多处 `ldc` 指令引用（Java 常量池去重），
/// 因此一个索引可能对应多个源码行号。
pub fn locate_constant_indices(class_bytes: &[u8], jdk_path: &str) -> Result<std::collections::HashMap<u16, Vec<usize>>, DecompileError> {
    let javap_bin = javap_executable(jdk_path)?;

    let temp_dir = std::env::temp_dir();
    let class_stem = format!("editclass_{}", random_id());
    let temp_class = temp_dir.join(format!("{}.class", class_stem));
    {
        let mut f = fs::File::create(&temp_class)?;
        f.write_all(class_bytes)?;
    }

    let output = new_command(&javap_bin)
        .arg("-J-Dfile.encoding=UTF-8")
        .arg("-c")
        .arg("-l")
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

    let text = decode_text(&output.stdout);
    Ok(parse_javap_index_to_lines(&text))
}

/// LineNumberTable 条目：(起始偏移, 行号)
struct LineNumberEntry {
    start_pc: usize,
    line: usize,
}

/// 解析 javap -c -l 输出，提取常量池索引 → 源码行号数组映射。
///
/// 解析流程（每个方法内独立处理）：
/// 1. 先收集所有 ldc/ldc_w 指令：(字节码偏移, 常量池索引)
/// 2. 再收集 LineNumberTable：(起始偏移, 行号)
/// 3. LineNumberTable 结束时，将每个 ldc 偏移映射到对应行号
fn parse_javap_index_to_lines(text: &str) -> std::collections::HashMap<u16, Vec<usize>> {
    let mut map: std::collections::HashMap<u16, Vec<usize>> = std::collections::HashMap::new();
    let mut current_ldcs: Vec<(usize, u16)> = Vec::new();
    let mut current_lnt: Vec<LineNumberEntry> = Vec::new();
    let mut in_line_number_table = false;

    // 处理完一个方法时，根据 ldc 偏移和 LineNumberTable 计算行号映射
    let flush_method = |ldcs: &[(usize, u16)],
                        lnt: &[LineNumberEntry],
                        out: &mut std::collections::HashMap<u16, Vec<usize>>| {
        if lnt.is_empty() {
            return;
        }
        for &(ldc_pc, const_idx) in ldcs {
            // 找到 ldc_pc 落在哪个 LNT 区间
            // LNT 按 start_pc 升序，取 start_pc <= ldc_pc 的最大条目
            let mut line = lnt[0].line;
            for entry in lnt {
                if entry.start_pc <= ldc_pc {
                    line = entry.line;
                } else {
                    break;
                }
            }
            out.entry(const_idx).or_default().push(line);
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();

        // 方法分隔：遇到新方法签名（非 Code/LineNumberTable/注释/字节码行）时 flush
        // 如果检测到 "Code:"、"LineNumberTable:" 或字节码指令行，不 flush
        let is_code_or_table = trimmed.contains("Code:")
            || trimmed.contains("LineNumberTable:");
        let is_bytecode_line = trimmed
            .find(':')
            .map(|pos| trimmed[..pos].trim().chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(false);

        if !trimmed.is_empty()
            && !is_code_or_table
            && !is_bytecode_line
            && !trimmed.starts_with("line ")
            && !trimmed.starts_with("LocalVariableTable:")
            && !trimmed.starts_with("stack=")
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("Exception table:")
        {
            // 可能是新方法或其他分隔，flush 上一个方法的数据
            if !current_ldcs.is_empty() {
                flush_method(&current_ldcs, &current_lnt, &mut map);
            }
            current_ldcs.clear();
            current_lnt.clear();
            in_line_number_table = false;
            continue;
        }

        // 检测 LineNumberTable 段开始
        if trimmed.contains("LineNumberTable:") {
            in_line_number_table = true;
            continue;
        }

        if in_line_number_table {
            if let Some(rest) = trimmed.strip_prefix("line ") {
                if let Some(colon_pos) = rest.find(':') {
                    let line_num: usize = rest[..colon_pos].trim().parse().unwrap_or(0);
                    let offset: usize = rest[colon_pos + 1..].trim().parse().unwrap_or(0);
                    current_lnt.push(LineNumberEntry {
                        start_pc: offset,
                        line: line_num,
                    });
                    current_lnt.sort_by_key(|e| e.start_pc);
                    continue;
                }
            }
            // LineNumberTable 段结束（下一个 line 条目不匹配）
            in_line_number_table = false;
            // 注意：不要在此 flush，方法结束时统一 flush（可能还有 LocalVariableTable）
        }

        // 解析字节码指令行
        if is_bytecode_line {
            if let Some(colon_pos) = trimmed.find(':') {
                let offset_str = trimmed[..colon_pos].trim();
                let offset: usize = offset_str.parse().unwrap_or(0);
                let after_colon = trimmed[colon_pos + 1..].trim();

                // ldc / ldc_w / ldc2_w 均可能引用字符串常量池索引
                // ldc_w 用于 >255 的索引，ldc2_w 用于 long/double（不涉及字符串）
                if after_colon.starts_with("ldc") && !after_colon.starts_with("ldc2") {
                    if let Some(hash_pos) = after_colon.find('#') {
                        let after_hash = &after_colon[hash_pos + 1..];
                        let num_str: String = after_hash
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if let Ok(const_idx) = num_str.parse::<u16>() {
                            current_ldcs.push((offset, const_idx));
                        }
                    }
                }
            }
        }
    }

    // flush 最后一个方法
    if !current_ldcs.is_empty() {
        flush_method(&current_ldcs, &current_lnt, &mut map);
    }

    // 对每个索引的行号数组去重并排序
    for lines in map.values_mut() {
        lines.sort_unstable();
        lines.dedup();
    }

    map
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

/// 创建子进程命令，Windows 上设置 CREATE_NO_WINDOW 标志避免弹出黑窗口。
fn new_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW = 0x08000000，阻止为子进程创建控制台窗口
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}
