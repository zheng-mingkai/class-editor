//! Class 文件解析器。
//!
//! 完整解析常量池（支持编辑），常量池之后的字节以原始 blob 保留，
//! 因为后续结构仅通过索引引用常量池，修改 Utf8 值不影响索引。

use super::constant_pool::{ConstantEntry, ConstantPool, ConstantTag};
use super::mutf8;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("无效的 class 文件 magic: 0x{0:08X}")]
    BadMagic(u32),
    #[error("文件截断: {0}")]
    Truncated(&'static str),
    #[error("未知的常量池标签: {0}")]
    UnknownTag(u8),
    #[error("无效的常量池索引: {0}")]
    BadIndex(u16),
    #[error("modified UTF-8 解码失败: {0}")]
    MUtf8(#[from] mutf8::MUtf8Error),
}

/// 解析后的 class 文件。
#[derive(Debug, Clone, Serialize)]
pub struct ClassFile {
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: ConstantPool,
    /// 常量池之后的全部原始字节（access_flags 起到文件末尾）。
    pub tail_bytes: Vec<u8>,
}

/// 读取器游标。
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    fn u1(&mut self) -> Result<u8, ParseError> {
        let b = self.data.get(self.pos).copied().ok_or(ParseError::Truncated("u1"))?;
        self.pos += 1;
        Ok(b)
    }
    fn u2(&mut self) -> Result<u16, ParseError> {
        let s = self
            .data
            .get(self.pos..self.pos + 2)
            .ok_or(ParseError::Truncated("u2"))?;
        let v = u16::from_be_bytes([s[0], s[1]]);
        self.pos += 2;
        Ok(v)
    }
    fn u4(&mut self) -> Result<u32, ParseError> {
        let s = self
            .data
            .get(self.pos..self.pos + 4)
            .ok_or(ParseError::Truncated("u4"))?;
        let v = u32::from_be_bytes([s[0], s[1], s[2], s[3]]);
        self.pos += 4;
        Ok(v)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        let s = self
            .data
            .get(self.pos..self.pos + n)
            .ok_or(ParseError::Truncated("take"))?;
        self.pos += n;
        Ok(s)
    }
}

/// 解析 class 文件字节。
pub fn parse_classfile(bytes: &[u8]) -> Result<ClassFile, ParseError> {
    let mut cur = Cursor::new(bytes);

    let magic = cur.u4()?;
    if magic != 0xCAFEBABE {
        return Err(ParseError::BadMagic(magic));
    }
    let minor_version = cur.u2()?;
    let major_version = cur.u2()?;

    let constant_pool = parse_constant_pool(&mut cur)?;

    // 常量池之后的字节整体保留
    let tail_bytes = cur.data[cur.pos..].to_vec();

    Ok(ClassFile {
        minor_version,
        major_version,
        constant_pool,
        tail_bytes,
    })
}

fn parse_constant_pool(cur: &mut Cursor) -> Result<ConstantPool, ParseError> {
    let count = cur.u2()?;
    let mut pool = ConstantPool::new();
    // entries 已含索引 0 占位
    let mut i = 1u16;
    while i < count {
        let tag_byte = cur.u1()?;
        let tag = ConstantTag::from_u8(tag_byte).ok_or(ParseError::UnknownTag(tag_byte))?;
        let entry = parse_entry(cur, tag)?;
        let takes_two = tag.takes_two_slots();
        pool.entries.push(Some(entry));
        if takes_two {
            // Long/Double 占两个槽位，第二个槽位不可用
            pool.entries.push(None);
            i += 2;
        } else {
            i += 1;
        }
    }
    Ok(pool)
}

fn parse_entry(cur: &mut Cursor, tag: ConstantTag) -> Result<ConstantEntry, ParseError> {
    match tag {
        ConstantTag::Utf8 => {
            let len = cur.u2()? as usize;
            let bytes = cur.take(len)?.to_vec();
            let value = mutf8::decode_mutf8(&bytes)?;
            Ok(ConstantEntry::Utf8 { value, raw_bytes: bytes })
        }
        ConstantTag::Long | ConstantTag::Double => {
            // 8 字节数据
            let bytes = cur.take(8)?.to_vec();
            Ok(ConstantEntry::WideLiteral { tag, bytes })
        }
        ConstantTag::Integer | ConstantTag::Float => {
            let bytes = cur.take(4)?.to_vec();
            Ok(ConstantEntry::Other { tag, bytes })
        }
        // 引用类型：2 字节索引
        ConstantTag::Class
        | ConstantTag::String
        | ConstantTag::MethodType
        | ConstantTag::Module
        | ConstantTag::Package => {
            let bytes = cur.take(2)?.to_vec();
            Ok(ConstantEntry::Other { tag, bytes })
        }
        // Fieldref/Methodref/InterfaceMethodref/NameAndType/Dynamic/InvokeDynamic: 4 字节
        ConstantTag::Fieldref
        | ConstantTag::Methodref
        | ConstantTag::InterfaceMethodref
        | ConstantTag::NameAndType
        | ConstantTag::Dynamic
        | ConstantTag::InvokeDynamic => {
            let bytes = cur.take(4)?.to_vec();
            Ok(ConstantEntry::Other { tag, bytes })
        }
        ConstantTag::MethodHandle => {
            // u1 reference_kind + u2 reference_index
            let bytes = cur.take(3)?.to_vec();
            Ok(ConstantEntry::Other { tag, bytes })
        }
    }
}
