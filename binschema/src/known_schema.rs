//! Trait for types which statically tell you the schema by which they'll
//! serde, and implementations for common types.


use crate::schema::*;
use std::{
    collections::{
        BinaryHeap,
        BTreeSet,
        HashSet,
        LinkedList,
        VecDeque,
        BTreeMap,
        HashMap,
    },
    ops::{
        Range,
        RangeInclusive,
        Bound,
    },
    borrow::Cow,
};


/// Type which know what `Schema` its `serde`s with.
pub trait KnownSchema {
    fn schema() -> Schema;
}


macro_rules! scalars_known_schema {
    ($($t:tt,)*)=>{$(
        impl KnownSchema for $t {
            fn schema() -> Schema {
                schema!($t)
            }
        }
    )*};
}

scalars_known_schema!(
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64,
    char,
    bool,
);

impl KnownSchema for usize {
    fn schema() -> Schema {
        schema!(u64)
    }
}

impl KnownSchema for isize {
    fn schema() -> Schema {
        schema!(i64)
    }
}

impl KnownSchema for str {
    fn schema() -> Schema {
        schema!(str)
    }
}

impl KnownSchema for String {
    fn schema() -> Schema {
        schema!(str)
    }
}

impl KnownSchema for () {
    fn schema() -> Schema {
        schema!(unit)
    }
}

impl<T: KnownSchema> KnownSchema for Option<T> {
    fn schema() -> Schema {
        schema!(option(%T::schema()))
    }
}

macro_rules! seqs_known_schema {
    ($($c:ident,)*)=>{$(
        impl<T: KnownSchema> KnownSchema for $c<T> {
            fn schema() -> Schema {
                schema!(seq(varlen)(%T::schema()))
            }
        }
    )*};
}

seqs_known_schema!(
    Vec,
    BinaryHeap,
    BTreeSet,
    HashSet,
    LinkedList,
    VecDeque,
);

macro_rules! maps_known_schema {
    ($($c:ident,)*)=>{$(
        impl<K: KnownSchema, V: KnownSchema> KnownSchema for $c<K, V> {
            fn schema() -> Schema {
                schema!(seq(varlen)(tuple {
                    (%K::schema()),
                    (%V::schema()),
                }))
            }
        }
    )*};
}

maps_known_schema!(
    BTreeMap,
    HashMap,
);

impl<T: KnownSchema, const LEN: usize> KnownSchema for [T; LEN] {
    fn schema() -> Schema {
        schema!(seq(LEN)(%T::schema()))
    }
}

impl<T: KnownSchema> KnownSchema for [T] {
    fn schema() -> Schema {
        schema!(seq(varlen)(%T::schema()))
    }
}

macro_rules! tuples_known_schema {
    (@inner $($t:ident),*)=>{
        impl<$($t: KnownSchema),*> KnownSchema for ($($t),*) {
            fn schema() -> Schema {
                schema!(tuple {$(
                    (%$t::schema()),
                )*})
            }
        }
    };
    ($a:ident, $b:ident $(, $t:ident)*)=>{
        tuples_known_schema!(@inner $a, $b $(, $t)*);
        tuples_known_schema!($b $(, $t)*);
    };
    ($a:ident)=>{};
}

tuples_known_schema!(A, B, C, D, E, F, G, H, I, J, K);

impl<T: KnownSchema> KnownSchema for Range<T> {
    fn schema() -> Schema {
        schema!(struct {
            (begin: %T::schema()),
            (end: %T::schema()),
        })
    }
}

impl<T: KnownSchema> KnownSchema for RangeInclusive<T> {
    fn schema() -> Schema {
        schema!(struct {
            (begin: %T::schema()),
            (end: %T::schema()),
        })
    }
}

impl<T: KnownSchema> KnownSchema for Bound<T> {
    fn schema() -> Schema {
        schema!(enum {
            Included(%T::schema()),
            Excluded(%T::schema()),
            Unbounded(unit),
        })
    }
}

impl<'a, T: KnownSchema> KnownSchema for &'a T {
    fn schema() -> Schema {
        T::schema()
    }
}

impl<'a, T: KnownSchema> KnownSchema for &'a mut T {
    fn schema() -> Schema {
        T::schema()
    }
}

impl<T: KnownSchema> KnownSchema for Box<T> {
    fn schema() -> Schema {
        T::schema()
    }
}

impl<'a, T: KnownSchema + ToOwned> KnownSchema for Cow<'a, T> {
    fn schema() -> Schema {
        T::schema()
    }
}

// TODO: unfortunately, there are more

impl KnownSchema for Schema {
    fn schema() -> Schema {
        schema!(enum {
            Scalar(enum {
                U8(unit),
                U16(unit),
                U32(unit),
                U64(unit),
                U128(unit),
                I8(unit),
                I16(unit),
                I32(unit),
                I64(unit),
                I128(unit),
                F32(unit),
                F64(unit),
                Char(unit),
                Bool(unit),
            }),
            Str(unit),
            Bytes(unit),
            Unit(unit),
            Option(recurse(1)),
            Seq(struct {
                (len: option(u64)),
                (inner: recurse(2)),
            }),
            Tuple(seq(varlen)(recurse(2))),
            Struct(seq(varlen)(struct {
                (name: str),
                (inner: recurse(3)),
            })),
            Enum(seq(varlen)(struct {
                (name: str),
                (inner: recurse(3)),
            })),
            Recurse(u64),
        })
    }
}
