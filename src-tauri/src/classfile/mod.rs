//! Class 文件解析/序列化模块入口。

pub mod constant_pool;
pub mod mutf8;
pub mod parser;
pub mod serializer;

pub use constant_pool::{ConstantEntry, ConstantTag};
pub use parser::{parse_classfile, ClassFile};
pub use serializer::serialize_classfile;
