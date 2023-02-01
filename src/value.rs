//! Dynamic representation of data within the serialized data model, analogous
//! to `serde_json::Value`.


#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Scalar(ScalarValue),
    CharString(String),
    ByteString(Vec<u8>),
    Unit,
    Option(Option<Box<Value>>),
    Seq(Vec<Value>),
    Struct(Vec<StructValueField>),
    Enum(EnumValue),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ScalarValue {
    U8(u8), U16(u16), U32(u32), U64(u64), U128(u128),
    I8(i8), I16(i16), I32(i32), I64(i64), I128(i128),
    F32(f32), F64(f64),
    Char(char),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct StructValueField {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct EnumValue {
    pub variant_ord: usize,
    pub variant_name: String,
    pub value: Box<Value>,
}
