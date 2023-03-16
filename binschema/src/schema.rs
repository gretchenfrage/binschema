//! Data types for representing a schema, and the macro for constructing them
//! with syntactic sugar.

use serde::{
    Serialize,
    Deserialize,
};


/// Description of how raw binary data encodes less tedious structures of
/// semantic primitives.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Schema {
    /// Some scalar data type.
    Scalar(ScalarType),
    /// Utf8 string.
    Str,
    /// Byte string.
    Bytes,
    /// Unit (0 bytes).
    Unit,
    /// Option (some or none).
    Option(Box<Schema>),
    /// Homogenous sequence. May be fixed or variable length.
    Seq(SeqSchema),
    /// Heterogenous fixed-length sequence.
    Tuple(Vec<Schema>),
    /// Sequence fields with names and ordinals.
    Struct(Vec<StructSchemaField>),
    /// Tagged union of variants with names and ordinals.
    Enum(Vec<EnumSchemaVariant>),
    /// Recurse type. This allows schema to be self-referential.
    ///
    /// Represents a reference to the type n layers above self in the schema
    /// tree. So for eg, a binary search tree could be represented as:
    ///
    /// ```
    /// use binschema::schema::{Schema, ScalarType};
    ///
    /// Schema::Enum(vec![
    ///     ("Branch", Schema::Struct(vec![
    ///         ("left", Schema::Recurse(2)).into(),
    ///         ("right", Schema::Recurse(2)).into(),
    ///     ])).into(),
    ///     ("Leaf", Schema::Scalar(ScalarType::I32)).into(),
    /// ]);
    /// ```
    ///
    /// `Recurse(0)` would recurse to itself, but it is illegal, as attempting
    /// to resolve leads to an infinite loop.
    Recurse(usize),
}

impl Schema {
    pub(crate) fn non_recursive_display_str(&self) -> &'static str {
        match self {
            Schema::Scalar(st) => st.display_str(),
            Schema::Str => "str",
            Schema::Bytes => "bytes",
            Schema::Unit => "unit",
            Schema::Option(_) => "option(..)",
            Schema::Seq(_) => "seq(..)(..)",
            Schema::Tuple(_)=> "tuple {..}",
            Schema::Struct(_) => "struct {..}",
            Schema::Enum(_) => "enum {..}",
            Schema::Recurse(_) => "recurse(_)",
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ScalarType {
    /// Encoded as-is.
    U8,
    /// Encoded little-endian.
    U16,
    /// Encoded var len.
    U32,
    /// Encoded var len.
    U64,
    /// Encoded var len.
    U128,
    /// Encoded as-is.
    I8,
    /// Encoded little-endian.
    I16,
    /// Encoded var len.
    I32,
    /// Encoded var len.
    I64,
    /// Encoded var len.
    I128,
    /// Encoded little-endian.
    F32,
    /// Encoded little-endian.
    F64,
    Char,
    /// Encoded as 1 byte, 0 or 1.
    Bool,
}

impl ScalarType {
    fn display_str(self) -> &'static str {
        match self {
            ScalarType::U8 => "u8",
            ScalarType::U16 => "u16",
            ScalarType::U32 => "u32",
            ScalarType::U64 => "u64",
            ScalarType::U128 => "u128",
            ScalarType::I8 => "i8",
            ScalarType::I16 => "i16",
            ScalarType::I32 => "i32",
            ScalarType::I64 => "i64",
            ScalarType::I128 => "i128",
            ScalarType::F32 => "f32",
            ScalarType::F64 => "f64",
            ScalarType::Char => "char",
            ScalarType::Bool => "bool",
        }
    }
}

/// Value in `Schema::Seq`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SeqSchema {
    pub len: Option<usize>,
    pub inner: Box<Schema>,
}

/// Item in `Schema::Struct`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct StructSchemaField {
    pub name: String,
    pub inner: Schema,
}

impl<S: Into<String>> From<(S, Schema)> for StructSchemaField {
    fn from((name, inner): (S, Schema)) -> Self {
        StructSchemaField {
            name: name.into(),
            inner,
        }
    }
}

/// Item in `Schema::Enum`. 
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct EnumSchemaVariant {
    pub name: String,
    pub inner: Schema,
}

impl<S: Into<String>> From<(S, Schema)> for EnumSchemaVariant {
    fn from((name, inner): (S, Schema)) -> Self {
        EnumSchemaVariant {
            name: name.into(),
            inner,
        }
    }
}

#[macro_export]
macro_rules! schema {
    (u8)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::U8) };
    (u16)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::U16) };
    (u32)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::U32) };
    (u64)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::U64) };
    (u128)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::U128) };
    (i8)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::I8) };
    (i16)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::I16) };
    (i32)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::I32) };
    (i64)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::I64) };
    (i128)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::I128) };
    (f32)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::F32) };
    (f64)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::F64) };
    (char)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::Char) };
    (bool)=>{ $crate::schema::Schema::Scalar($crate::schema::ScalarType::Bool) };
    (str)=>{ $crate::schema::Schema::Str };
    (bytes)=>{ $crate::schema::Schema::Bytes };
    (unit)=>{ $crate::schema::Schema::Unit };
    (option($($inner:tt)*))=>{ $crate::schema::Schema::Option(::std::boxed::Box::new($crate::schema!($($inner)*))) };
    (seq(varlen)($($inner:tt)*))=>{ $crate::schema::Schema::Seq($crate::schema::SeqSchema { len: ::core::option::Option::None, inner: ::std::boxed::Box::new($crate::schema!($($inner)*)) }) };
    (seq($len:expr)($($inner:tt)*))=>{ $crate::schema::Schema::Seq($crate::schema::SeqSchema { len: ::core::option::Option::Some($len), inner: ::std::boxed::Box::new($crate::schema!($($inner)*)) }) };
    (tuple { $(($($item:tt)*)),*$(,)? })=>{ $crate::schema::Schema::Tuple(::std::vec![$( $crate::schema!($($item)*), )*]) };
    (struct { $(($name:ident: $($field:tt)*)),*$(,)? })=>{ $crate::schema::Schema::Struct(::std::vec![$( $crate::schema::StructSchemaField { name: ::std::string::String::from(::core::stringify!($name)), inner: $crate::schema!($($field)*) }, )*]) };
    (enum { $($name:ident($($variant:tt)*)),*$(,)? })=>{ $crate::schema::Schema::Enum(::std::vec![$( $crate::schema::EnumSchemaVariant { name: ::std::string::String::from(::core::stringify!($name)), inner: $crate::schema!($($variant)*) }, )*]) };
    (recurse($n:expr))=>{ $crate::schema::Schema::Recurse($n) };
    (%$schema:expr)=>{ $schema };
}

pub use schema;
