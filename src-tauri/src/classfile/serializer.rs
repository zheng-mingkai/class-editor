//! Class 文件序列化器：将解析后的结构重新输出为字节流。
//!
//! 由于常量池是变长的，修改 Utf8 值后必须整体重写。顺序写入保证偏移正确。

use super::constant_pool::{ConstantEntry, ConstantPool, ConstantTag};
use super::parser::ClassFile;

/// 序列化 class 文件。
pub fn serialize_classfile(cf: &ClassFile) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + cf.tail_bytes.len());
    // magic
    out.extend_from_slice(&0xCAFEBABE_u32.to_be_bytes());
    // version
    out.extend_from_slice(&cf.minor_version.to_be_bytes());
    out.extend_from_slice(&cf.major_version.to_be_bytes());
    // constant pool
    serialize_constant_pool(&cf.constant_pool, &mut out);
    // 常量池之后的原始字节
    out.extend_from_slice(&cf.tail_bytes);
    out
}

fn serialize_constant_pool(pool: &ConstantPool, out: &mut Vec<u8>) {
    // count = entries 长度（含索引 0 占位）
    out.extend_from_slice(&pool.count().to_be_bytes());
    for entry in pool.entries.iter().skip(1) {
        match entry {
            Some(ConstantEntry::Utf8 { raw_bytes, .. }) => {
                out.push(ConstantTag::Utf8 as u8);
                out.extend_from_slice(&(raw_bytes.len() as u16).to_be_bytes());
                out.extend_from_slice(raw_bytes);
            }
            Some(ConstantEntry::WideLiteral { tag, bytes }) => {
                out.push(*tag as u8);
                out.extend_from_slice(bytes);
            }
            Some(ConstantEntry::Other { tag, bytes }) => {
                out.push(*tag as u8);
                out.extend_from_slice(bytes);
            }
            None => {
                // Long/Double 占用的第二个槽位，不输出任何字节
            }
        }
    }
}
