//! JDK 检测与路径管理。

use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
pub struct JdkInfo {
    pub path: String,
    pub version: String,
    pub source: JdkSource,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JdkSource {
    /// JAVA_HOME 环境变量
    Env,
    /// 操作系统常见路径
    System,
    /// 用户自定义
    Custom,
}

/// 检测 JDK：优先 JAVA_HOME，回退到系统常见路径。
pub fn detect_jdk() -> Option<JdkInfo> {
    // 1. JAVA_HOME
    if let Ok(home) = std::env::var("JAVA_HOME") {
        if let Some(info) = try_jdk(&PathBuf::from(&home), JdkSource::Env) {
            return Some(info);
        }
    }
    // 2. 系统常见路径
    for path in system_jdk_paths() {
        if let Some(info) = try_jdk(&path, JdkSource::System) {
            return Some(info);
        }
    }
    None
}

/// 验证给定路径是否为有效 JDK，返回其版本。
pub fn validate_jdk(path: &str) -> Option<JdkInfo> {
    try_jdk(&PathBuf::from(path), JdkSource::Custom)
}

fn try_jdk(home: &PathBuf, source: JdkSource) -> Option<JdkInfo> {
    let java_bin = home.join("bin").join("java");
    let java_bin = if cfg!(windows) {
        if java_bin.with_extension("exe").exists() {
            java_bin.with_extension("exe")
        } else {
            java_bin
        }
    } else {
        java_bin
    };
    if !java_bin.exists() {
        return None;
    }
    let version = parse_java_version(&java_bin)?;
    Some(JdkInfo {
        path: home.to_string_lossy().to_string(),
        version,
        source,
    })
}

fn parse_java_version(java_bin: &PathBuf) -> Option<String> {
    let output = new_command(java_bin).arg("-version").output().ok()?;
    // java -version 输出到 stderr
    let text = String::from_utf8_lossy(&output.stderr);
    for line in text.lines() {
        if line.contains("version") {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    None
}

/// 创建子进程命令，Windows 上设置 CREATE_NO_WINDOW 标志避免弹出黑窗口。
fn new_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// 返回各操作系统常见 JDK 安装路径。
fn system_jdk_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if cfg!(target_os = "windows") {
        if let Ok(p) = std::env::var("ProgramFiles") {
            let base = PathBuf::from(p).join("Java");
            if let Ok(entries) = std::fs::read_dir(&base) {
                for e in entries.flatten() {
                    paths.push(e.path());
                }
            }
        }
    } else if cfg!(target_os = "macos") {
        let base = PathBuf::from("/Library/Java/JavaVirtualMachines");
        if let Ok(entries) = std::fs::read_dir(&base) {
            for e in entries.flatten() {
                paths.push(e.path().join("Contents").join("Home"));
            }
        }
    } else if cfg!(target_os = "linux") {
        let base = PathBuf::from("/usr/lib/jvm");
        if let Ok(entries) = std::fs::read_dir(&base) {
            for e in entries.flatten() {
                paths.push(e.path());
            }
        }
    }
    paths
}

/// 配置文件管理：将自定义 JDK 路径存到 app data 目录。
pub fn load_config(app: &AppHandle) -> Option<String> {
    let path = config_path(app)?;
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("jdk_path").and_then(|p| p.as_str()).map(String::from))
}

pub fn save_config(app: &AppHandle, jdk_path: &str) -> Result<(), String> {
    let path = config_path(app).ok_or("无法获取 app data 目录")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let v = serde_json::json!({ "jdk_path": jdk_path });
    std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap())
        .map_err(|e| e.to_string())
}

fn config_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join("settings.json"))
}
