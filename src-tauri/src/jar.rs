//! JAR/ZIP 读取、目录树构建、单条目替换、签名检测。

use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Error)]
pub enum JarError {
    #[error("无法打开 JAR 文件: {0}")]
    Open(String),
    #[error("JAR 条目不存在: {0}")]
    EntryNotFound(String),
    #[error("ZIP 读取错误: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct JarEntry {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<FileTreeNode>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JarInfo {
    pub path: String,
    pub entries: Vec<JarEntry>,
    pub file_tree: FileTreeNode,
    pub is_signed: bool,
    pub manifest: Option<String>,
}

/// 列出 JAR 中所有条目。
pub fn list_jar_entries(path: &str) -> Result<Vec<JarEntry>, JarError> {
    let file = fs::File::open(path).map_err(|e| JarError::Open(e.to_string()))?;
    let mut archive = ZipArchive::new(file)?;
    let mut entries = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i)?;
        entries.push(JarEntry {
            name: entry.name().to_string(),
            size: entry.size(),
            compressed_size: entry.compressed_size(),
            is_dir: entry.is_dir(),
        });
    }
    Ok(entries)
}

/// 将扁平条目列表构建为目录树。
pub fn build_file_tree(entries: &[JarEntry]) -> FileTreeNode {
    let mut root = FileTreeNode {
        name: String::new(),
        path: String::new(),
        is_dir: true,
        children: Vec::new(),
        size: None,
    };
    for entry in entries {
        let parts: Vec<&str> = entry.name.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }
        insert_node(&mut root, &parts, entry);
    }
    root
}

fn insert_node(node: &mut FileTreeNode, parts: &[&str], entry: &JarEntry) {
    if parts.is_empty() {
        return;
    }
    let name = parts[0].to_string();
    let is_last = parts.len() == 1;
    // 查找或创建子节点
    let idx = node.children.iter().position(|c| c.name == name);
    if is_last {
        if let Some(i) = idx {
            // 更新现有节点信息
            node.children[i].is_dir = entry.is_dir;
            node.children[i].size = if entry.is_dir { None } else { Some(entry.size) };
        } else {
            node.children.push(FileTreeNode {
                name: name.clone(),
                path: entry.name.clone(),
                is_dir: entry.is_dir,
                children: Vec::new(),
                size: if entry.is_dir { None } else { Some(entry.size) },
            });
        }
    } else {
        let path = if node.path.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", node.path, name)
        };
        if idx.is_none() {
            node.children.push(FileTreeNode {
                name: name.clone(),
                path: path.clone(),
                is_dir: true,
                children: Vec::new(),
                size: None,
            });
        }
        let i = node.children.len() - 1;
        insert_node(&mut node.children[i], &parts[1..], entry);
    }
}

/// 读取 JAR 中指定条目的字节。
pub fn read_entry(jar_path: &str, entry_name: &str) -> Result<Vec<u8>, JarError> {
    let file = fs::File::open(jar_path).map_err(|e| JarError::Open(e.to_string()))?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry = archive
        .by_name(entry_name)
        .map_err(|e| match e {
            zip::result::ZipError::FileNotFound => JarError::EntryNotFound(entry_name.to_string()),
            other => JarError::Zip(other),
        })?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

/// 检查 JAR 是否已签名（META-INF 下存在 *.SF|*.DSA|*.RSA）。
pub fn check_signed(jar_path: &str) -> Result<bool, JarError> {
    let file = fs::File::open(jar_path).map_err(|e| JarError::Open(e.to_string()))?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i)?;
        let name = entry.name();
        if name.starts_with("META-INF/")
            && (name.ends_with(".SF") || name.ends_with(".DSA") || name.ends_with(".RSA"))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 读取 MANIFEST.MF 内容（若存在）。
pub fn read_manifest(jar_path: &str) -> Result<Option<String>, JarError> {
    let file = fs::File::open(jar_path).map_err(|e| JarError::Open(e.to_string()))?;
    let mut archive = ZipArchive::new(file)?;
    // 先绑定到局部变量，确保 ZipFile 临时值在返回前被释放（避免借用生命周期错误）
    let result = match archive.by_name("META-INF/MANIFEST.MF") {
        Ok(mut entry) => {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            Ok(Some(String::from_utf8_lossy(&buf).to_string()))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(e) => Err(JarError::Zip(e)),
    };
    result
}

/// 完整读取 JAR 信息。
pub fn read_jar_info(jar_path: &str) -> Result<JarInfo, JarError> {
    let entries = list_jar_entries(jar_path)?;
    let file_tree = build_file_tree(&entries);
    let is_signed = check_signed(jar_path)?;
    let manifest = read_manifest(jar_path)?;
    Ok(JarInfo {
        path: jar_path.to_string(),
        entries,
        file_tree,
        is_signed,
        manifest,
    })
}

/// 单条目替换：仅替换目标条目，其余条目原样保留。原子写入。
pub fn replace_entry(jar_path: &str, entry_name: &str, data: &[u8]) -> Result<(), JarError> {
    // 先创建备份
    let backup = format!("{}.bak", jar_path);
    fs::copy(jar_path, &backup)?;

    let tmp_path = format!("{}.tmp", jar_path);
    {
        let file = fs::File::open(jar_path)?;
        let mut archive = ZipArchive::new(file)?;
        let tmp_file = fs::File::create(&tmp_path)?;
        let mut writer = zip::ZipWriter::new(tmp_file);
        let opts: zip::write::SimpleFileOptions =
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            let is_dir = entry.is_dir();
            if name == entry_name {
                // 替换目标条目
                writer.start_file(&name, opts)?;
                writer.write_all(data)?;
            } else if is_dir {
                writer.add_directory(&name, opts)?;
            } else {
                // 原样复制（by_index 已解压，重新压缩写入）
                writer.start_file(&name, opts)?;
                let mut buf = Vec::with_capacity(entry.size() as usize);
                entry.read_to_end(&mut buf)?;
                writer.write_all(&buf)?;
            }
        }
        writer.finish()?;
    }

    // 原子替换
    fs::rename(&tmp_path, jar_path)?;
    Ok(())
}
