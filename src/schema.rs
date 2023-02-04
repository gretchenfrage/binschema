//! Data types for representing a schema, and the macro for constructing them
//! with syntactic sugar.


/// Description of how raw binary data encodes less tedious structures of
/// semantic primitives.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
    /// use serde_schema::schema::*;
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

/// Value in `Schema::Seq`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SeqSchema {
    pub len: Option<usize>,
    pub inner: Box<Schema>,
}

/// Item in `Schema::Struct`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

/// Syntax sugar for constructing `Schema`.
///
/// ```
/// use serde_schema::schema::*;
///
/// let _: Schema = schema!(u8);
/// let _: Schema = schema!(u16);
/// let _: Schema = schema!(u32);
/// let _: Schema = schema!(u64);
/// let _: Schema = schema!(u128);
/// let _: Schema = schema!(i8);
/// let _: Schema = schema!(i16);
/// let _: Schema = schema!(i32);
/// let _: Schema = schema!(i64);
/// let _: Schema = schema!(i128);
/// let _: Schema = schema!(f32);
/// let _: Schema = schema!(f64);
/// let _: Schema = schema!(char);
/// let _: Schema = schema!(bool);
/// let _: Schema = schema!(str);
/// let _: Schema = schema!(bytes);
/// let _: Schema = schema!(());
/// let _: Schema = schema!(?(str));
/// let _: Schema = schema!([7; str]);
/// let _: Schema = schema!([_; str]);
/// let _: Schema = schema!((
///     (i32),
///     (str),
/// ));
/// let _: Schema = schema!({
///     (foo: i32),
///     (bar: str),
/// });
/// let _: Schema = schema!(enum {
///     Foo(i32),
///     Bar(str),
/// });
/// let _binary_search_tree = schema!(enum {
///     Branch({
///         (left: recurse(2)),
///         (right: recurse(2)),
///     }),
///     Leaf(i32),
/// });
/// let _: Schema = schema!(%Schema::Str);
/// ```
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
    (())=>{ $crate::schema::Schema::Unit };
    (?($($inner:tt)*))=>{ $crate::schema::Schema::Option(::std::boxed::Box::new($crate::schema!($($inner)*))) };
    ([$len:expr; $($inner:tt)*])=>{ $crate::schema::Schema::Seq($crate::schema::SeqSchema { len: ::core::option::Option::Some($len), inner: ::std::boxed::Box::new($crate::schema!($($inner)*)) }) };
    ([_; $($inner:tt)*])=>{ $crate::schema::Schema::Seq($crate::schema::SeqSchema { len: ::core::option::Option::None, inner: ::std::boxed::Box::new($crate::schema!($($inner)*)) }) };
    (($(($($item:tt)*)),*$(,)?))=>{ $crate::schema::Schema::Tuple(::std::vec![$( $crate::schema!($($item)*), )*]) };
    ({ $(($name:ident: $($field:tt)*)),*$(,)? })=>{ $crate::schema::Schema::Struct(::std::vec![$( $crate::schema::StructSchemaField { name: ::std::string::String::from(::core::stringify!($name)), inner: $crate::schema!($($field)*) }, )*]) };
    (enum { $($name:ident($($variant:tt)*)),*$(,)? })=>{ $crate::schema::Schema::Enum(::std::vec![$( $crate::schema::EnumSchemaVariant { name: ::std::string::String::from(::core::stringify!($name)), inner: $crate::schema!($($variant)*) }, )*]) };
    (recurse($n:expr))=>{ $crate::schema::Schema::Recurse($n) };
    (%$schema:expr)=>{ $schema };
}

pub use schema;
