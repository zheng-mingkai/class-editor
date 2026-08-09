//! 常量池结构与 Utf8 条目处理。

use serde::Serialize;

/// 常量池条目类型标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum ConstantTag {
    Utf8 = 1,
    Integer = 3,
    Float = 4,
    Long = 5,
    Double = 6,
    Class = 7,
    String = 8,
    Fieldref = 9,
    Methodref = 10,
    InterfaceMethodref = 11,
    NameAndType = 12,
    MethodHandle = 15,
    MethodType = 16,
    Dynamic = 17,
    InvokeDynamic = 18,
    Module = 19,
    Package = 20,
}

impl ConstantTag {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::Utf8,
            3 => Self::Integer,
            4 => Self::Float,
            5 => Self::Long,
            6 => Self::Double,
            7 => Self::Class,
            8 => Self::String,
            9 => Self::Fieldref,
            10 => Self::Methodref,
            11 => Self::InterfaceMethodref,
            12 => Self::NameAndType,
            15 => Self::MethodHandle,
            16 => Self::MethodType,
            17 => Self::Dynamic,
            18 => Self::InvokeDynamic,
            19 => Self::Module,
            20 => Self::Package,
            _ => return None,
        })
    }

    /// 该条目是否占用两个常量池槽位（Long/Double）。
    pub fn takes_two_slots(self) -> bool {
        matches!(self, Self::Long | Self::Double)
    }
}

/// 常量池条目。仅完整实现需要修改的 Utf8 与引用 Utf8 的类型，
/// 其余类型以原始字节保留，保证序列化后与原文件一致。
#[derive(Debug, Clone, Serialize)]
pub enum ConstantEntry {
    /// CONSTANT_Utf8_info —— 可编辑的字符串条目。
    Utf8 { value: String, raw_bytes: Vec<u8> },
    /// 占两个槽位的字面量（Long/Double），以原始字节保留。
    WideLiteral { tag: ConstantTag, bytes: Vec<u8> },
    /// 单槽位、非 Utf8 条目，以原始字节保留（tag 之后的部分）。
    Other { tag: ConstantTag, bytes: Vec<u8> },
}

impl ConstantEntry {
    pub fn tag(&self) -> ConstantTag {
        match self {
            ConstantEntry::Utf8 { .. } => ConstantTag::Utf8,
            ConstantEntry::WideLiteral { tag, .. } => *tag,
            ConstantEntry::Other { tag, .. } => *tag,
        }
    }

    pub fn as_utf8_value(&self) -> Option<&str> {
        match self {
            ConstantEntry::Utf8 { value, .. } => Some(value),
            _ => None,
        }
    }

    /// 更新 Utf8 条目的值（同步刷新原始字节）。
    pub fn set_utf8_value(&mut self, new_value: String) {
        if let ConstantEntry::Utf8 { value, raw_bytes } = self {
            *raw_bytes = crate::classfile::mutf8::encode_mutf8(&new_value);
            *value = new_value;
        }
    }
}

/// 常量池：1-based 索引，索引 0 保留。Long/Double 占用两个槽位，第二个槽位为 None。
#[derive(Debug, Clone, Serialize, Default)]
pub struct ConstantPool {
    /// 索引 0 不使用，使用 None 占位。
    pub entries: Vec<Option<ConstantEntry>>,
}

impl ConstantPool {
    pub fn new() -> Self {
        Self {
            entries: vec![None],
        }
    }

    pub fn count(&self) -> u16 {
        // count 等于 entries 长度（含索引 0 的占位）
        self.entries.len() as u16
    }

    pub fn get(&self, index: u16) -> Option<&ConstantEntry> {
        self.entries
            .get(index as usize)
            .and_then(|e| e.as_ref())
    }

    pub fn get_mut(&mut self, index: u16) -> Option<&mut ConstantEntry> {
        self.entries
            .get_mut(index as usize)
            .and_then(|e| e.as_mut())
    }

    /// 返回所有被 CONSTANT_String_info 引用的 Utf8 索引集合（即"字面量"）。
    pub fn literal_utf8_indices(&self) -> std::collections::HashSet<u16> {
        let mut set = std::collections::HashSet::new();
        for e in self.entries.iter().flatten() {
            if let ConstantEntry::Other {
                tag: ConstantTag::String,
                bytes,
            } = e
            {
                // String_info: u2 string_index（大端）
                if bytes.len() >= 2 {
                    let idx = u16::from_be_bytes([bytes[0], bytes[1]]);
                    set.insert(idx);
                }
            }
        }
        set
    }

    /// 返回所有 Utf8 条目的 (索引, 值) 列表。
    pub fn utf8_entries(&self) -> Vec<(u16, &str)> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                e.as_ref()
                    .and_then(|e| e.as_utf8_value().map(|v| (i as u16, v)))
            })
            .collect()
    }
}
