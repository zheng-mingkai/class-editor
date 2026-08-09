//! Modified UTF-8 编解码（Java class 文件常量池使用）。
//!
//! 与标准 UTF-8 的区别：
//! - `\u0000` 编码为 `0xC0 0x80`（双字节）
//! - 辅助平面字符（>= U+10000）以代理对形式编码，每个代理项占 3 字节

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MUtf8Error {
    #[error("无效的 modified UTF-8 字节序列: {0}")]
    InvalidBytes(String),
    #[error("字符串包含无法编码的字符")]
    InvalidString,
}

/// 将 modified UTF-8 字节解码为 Rust String。
pub fn decode_mutf8(bytes: &[u8]) -> Result<String, MUtf8Error> {
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b1 = bytes[i];
        if b1 < 0x80 {
            // 单字节：0x01-0x7F（0x00 不应直接出现，但容错处理）
            out.push(b1 as char);
            i += 1;
        } else if (b1 & 0xE0) == 0xC0 {
            // 双字节
            if i + 1 >= bytes.len() {
                return Err(MUtf8Error::InvalidBytes("双字节序列截断".into()));
            }
            let b2 = bytes[i + 1];
            if (b2 & 0xC0) != 0x80 {
                return Err(MUtf8Error::InvalidBytes("无效的续字节".into()));
            }
            let cp = ((b1 as u32 & 0x1F) << 6) | (b2 as u32 & 0x3F);
            out.push(cp as u8 as char);
            i += 2;
        } else if (b1 & 0xF0) == 0xE0 {
            // 三字节
            if i + 2 >= bytes.len() {
                return Err(MUtf8Error::InvalidBytes("三字节序列截断".into()));
            }
            let b2 = bytes[i + 1];
            let b3 = bytes[i + 2];
            if (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80 {
                return Err(MUtf8Error::InvalidBytes("无效的续字节".into()));
            }
            let cp = ((b1 as u32 & 0x0F) << 12) | ((b2 as u32 & 0x3F) << 6) | (b3 as u32 & 0x3F);
            // 可能是代理项的一部分，先收集
            if (0xD800..=0xDBFF).contains(&cp) {
                // 高代理，期望后面跟一个低代理
                if i + 5 < bytes.len() && (bytes[i + 3] & 0xF0) == 0xE0 {
                    let b4 = bytes[i + 3];
                    let b5 = bytes[i + 4];
                    let b6 = bytes[i + 5];
                    if (b5 & 0xC0) == 0x80 && (b6 & 0xC0) == 0x80 {
                        let lo = ((b4 as u32 & 0x0F) << 12)
                            | ((b5 as u32 & 0x3F) << 6)
                            | (b6 as u32 & 0x3F);
                        if (0xDC00..=0xDFFF).contains(&lo) {
                            let full = 0x10000
                                + ((cp - 0xD800) << 10)
                                + (lo - 0xDC00);
                            if let Some(c) = char::from_u32(full) {
                                out.push(c);
                                i += 6;
                                continue;
                            }
                        }
                    }
                }
                // 无法配对，保留代理项字符
                out.push(cp as u8 as char);
                i += 3;
            } else {
                out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                i += 3;
            }
        } else {
            return Err(MUtf8Error::InvalidBytes(format!("非法首字节 0x{:02X}", b1)));
        }
    }
    Ok(out)
}

/// 将 Rust String 编码为 modified UTF-8 字节。
pub fn encode_mutf8(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        if cp == 0 {
            // \u0000 特殊编码
            out.push(0xC0);
            out.push(0x80);
        } else if cp <= 0x7F {
            out.push(cp as u8);
        } else if cp <= 0x7FF {
            out.push(0xC0 | ((cp >> 6) as u8));
            out.push(0x80 | ((cp & 0x3F) as u8));
        } else if cp <= 0xFFFF {
            // BMP 字符（含代理项范围，但 Rust char 不会是孤立代理项）
            out.push(0xE0 | ((cp >> 12) as u8));
            out.push(0x80 | (((cp >> 6) & 0x3F) as u8));
            out.push(0x80 | ((cp & 0x3F) as u8));
        } else {
            // 辅助平面字符：编码为代理对，每个代理 3 字节
            let off = cp - 0x10000;
            let hi = 0xD800 + (off >> 10);
            let lo = 0xDC00 + (off & 0x3FF);
            for cp in [hi, lo] {
                out.push(0xE0 | ((cp >> 12) as u8));
                out.push(0x80 | (((cp >> 6) & 0x3F) as u8));
                out.push(0x80 | ((cp & 0x3F) as u8));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ascii() {
        let s = "hello world";
        assert_eq!(decode_mutf8(&encode_mutf8(s)).unwrap(), s);
    }

    #[test]
    fn roundtrip_chinese() {
        let s = "你好，世界";
        assert_eq!(decode_mutf8(&encode_mutf8(s)).unwrap(), s);
    }

    #[test]
    fn null_char() {
        let bytes = encode_mutf8("a\u{0000}b");
        assert_eq!(bytes, vec![b'a', 0xC0, 0x80, b'b']);
        assert_eq!(decode_mutf8(&bytes).unwrap(), "a\u{0000}b");
    }

    #[test]
    fn supplementary() {
        let s = "𝄞music"; // U+1D11E
        assert_eq!(decode_mutf8(&encode_mutf8(s)).unwrap(), s);
    }
}
