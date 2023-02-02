//! Data types for representing a schema, and the macro for constructing them
//! with syntactic sugar.


/// Description of how raw binary data encodes less ambiguous structures of
/// semantic primitives.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Schema {
    /// A scalar data type, encoded little-endian.
    Scalar(ScalarType),
    /// A string (of characters).
    ///
    /// Encoded as:
    /// - u64 length
    /// - length bytes of UTF-8 data
    CharString,
    /// A binary string.
    ///
    /// Encoded as:
    /// - u64 length
    /// - length bytes of data
    ByteString,
    /// Unitary data type. Encoded as nothing.
    Unit,
    /// Option data type.
    ///
    /// Encoded as:
    /// - bool is_some
    /// - if is_some:
    ///     - inner data
    Option(Box<Schema>),
    /// Homogenous sequence. May be fixed or variable length.
    ///
    /// Encoded as:
    /// - if schema.len is none:
    ///     - u64 length
    /// - repeating length times:
    ///     - inner data
    Seq(SeqSchema),
    /// Heterogenous fixed-length sequence.
    ///
    /// Encoded as:
    /// - for each item:
    ///     - inner data
    Tuple(Vec<Schema>),
    /// Struct of fields with both names and ordinals.
    ///
    /// Encoded as:
    /// - for each field:
    ///     - inner data
    Struct(Vec<StructSchemaField>),
    /// Tagged union of variants with both names and ordinals.
    ///
    /// Encoded as:
    /// - u64 ordinal of variant
    /// - inner data
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
    Recurse(usize),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ScalarType {
    U8, U16, U32, U64, U128,
    I8, I16, I32, I64, I128,
    F32, F64,
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
/// let _: Schema = schema!(%Schema::CharString);
/// ```
#[macro_export]
macro_rules! schema {
    (u8)=>{ $crate::Schema::Scalar($crate::ScalarType::U8) };
    (u16)=>{ $crate::Schema::Scalar($crate::ScalarType::U16) };
    (u32)=>{ $crate::Schema::Scalar($crate::ScalarType::U32) };
    (u64)=>{ $crate::Schema::Scalar($crate::ScalarType::U64) };
    (u128)=>{ $crate::Schema::Scalar($crate::ScalarType::U128) };
    (i8)=>{ $crate::Schema::Scalar($crate::ScalarType::I8) };
    (i16)=>{ $crate::Schema::Scalar($crate::ScalarType::I16) };
    (i32)=>{ $crate::Schema::Scalar($crate::ScalarType::I32) };
    (i64)=>{ $crate::Schema::Scalar($crate::ScalarType::I64) };
    (i128)=>{ $crate::Schema::Scalar($crate::ScalarType::I128) };
    (f32)=>{ $crate::Schema::Scalar($crate::ScalarType::F32) };
    (f64)=>{ $crate::Schema::Scalar($crate::ScalarType::F64) };
    (char)=>{ $crate::Schema::Scalar($crate::ScalarType::Char) };
    (bool)=>{ $crate::Schema::Scalar($crate::ScalarType::Bool) };
    (str)=>{ $crate::Schema::CharString };
    (bytes)=>{ $crate::Schema::ByteString };
    (())=>{ $crate::Schema::Unit };
    (?($($inner:tt)*))=>{ $crate::Schema::Option(::std::boxed::Box::new($crate::schema!($($inner)*))) };
    ([$len:expr; $($inner:tt)*])=>{ $crate::Schema::Seq($crate::SeqSchema { len: ::core::option::Option::Some($len), inner: ::std::boxed::Box::new($crate::schema!($($inner)*)) }) };
    ([_; $($inner:tt)*])=>{ $crate::Schema::Seq($crate::SeqSchema { len: ::core::option::Option::None, inner: ::std::boxed::Box::new($crate::schema!($($inner)*)) }) };
    (($(($($item:tt)*)),*$(,)?))=>{ $crate::Schema::Tuple(::std::vec![$( $crate::schema!($($item)*), )*]) };
    ({ $(($name:ident: $($field:tt)*)),*$(,)? })=>{ $crate::Schema::Struct(::std::vec![$( $crate::StructSchemaField { name: ::std::string::String::from(::core::stringify!($name)), inner: $crate::schema!($($field)*) }, )*]) };
    (enum { $($name:ident($($variant:tt)*)),*$(,)? })=>{ $crate::Schema::Enum(::std::vec![$( $crate::EnumSchemaVariant { name: ::std::string::String::from(::core::stringify!($name)), inner: $crate::schema!($($variant)*) }, )*]) };
    (recurse($n:expr))=>{ $crate::Schema::Recurse($n) };
    (%$schema:expr)=>{ $schema };
}

pub use schema;
