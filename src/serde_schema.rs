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
pub trait SerdeSchema {
    fn schema() -> Schema;
}


macro_rules! scalars_serde_schema {
    ($($t:tt,)*)=>{$(
        impl SerdeSchema for $t {
            fn schema() -> Schema {
                schema!($t)
            }
        }
    )*};
}

scalars_serde_schema!(
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64,
    char,
    bool,
);

impl SerdeSchema for usize {
    fn schema() -> Schema {
        schema!(u64)
    }
}

impl SerdeSchema for isize {
    fn schema() -> Schema {
        schema!(u64)
    }
}

impl SerdeSchema for str {
    fn schema() -> Schema {
        schema!(str)
    }
}

impl SerdeSchema for String {
    fn schema() -> Schema {
        schema!(str)
    }
}

impl SerdeSchema for () {
    fn schema() -> Schema {
        schema!(())
    }
}

impl<T: SerdeSchema> SerdeSchema for Option<T> {
    fn schema() -> Schema {
        schema!(?(%T::schema()))
    }
}

macro_rules! seqs_serde_schema {
    ($($c:ident,)*)=>{$(
        impl<T: SerdeSchema> SerdeSchema for $c<T> {
            fn schema() -> Schema {
                schema!([_; %T::schema()])
            }
        }
    )*};
}

seqs_serde_schema!(
    Vec,
    BinaryHeap,
    BTreeSet,
    HashSet,
    LinkedList,
    VecDeque,
);

macro_rules! maps_serde_schema {
    ($($c:ident,)*)=>{$(
        impl<K: SerdeSchema, V: SerdeSchema> SerdeSchema for $c<K, V> {
            fn schema() -> Schema {
                schema!([_; (
                    (%K::schema()),
                    (%V::schema()),
                )])
            }
        }
    )*};
}

maps_serde_schema!(
    BTreeMap,
    HashMap,
);

impl<T: SerdeSchema, const LEN: usize> SerdeSchema for [T; LEN] {
    fn schema() -> Schema {
        schema!([LEN; %T::schema()])
    }
}

impl<T: SerdeSchema> SerdeSchema for [T] {
    fn schema() -> Schema {
        schema!([_; %T::schema()])
    }
}

macro_rules! tuples_serde_schema {
    (@inner $(t:ident),*)=>{
        impl<$($t: SerdeSchema),*> SerdeSchema for ($($t),*) {
            fn schema() -> Schema {
                schema!(($(
                    (%$t::schema()),
                )*))
            }
        }
    };
    ($a:ident, $b:ident $(, $t:ident)*)=>{
        tuples_serde_schema!(@inner $a, $b $(, $t)*);
        tuples_serde_schema!($b $(, $t)*);
    };
    ($($whatever:tt)*)=>{};
}

tuples_serde_schema!(A, B, C, D, E, F, G, H, I, J, K);

impl<T: SerdeSchema> SerdeSchema for Range<T> {
    fn schema() -> Schema {
        schema!({
            (begin: %T::schema()),
            (end: %T::schema()),
        })
    }
}

impl<T: SerdeSchema> SerdeSchema for RangeInclusive<T> {
    fn schema() -> Schema {
        schema!({
            (begin: %T::schema()),
            (end: %T::schema()),
        })
    }
}

impl<T: SerdeSchema> SerdeSchema for Bound<T> {
    fn schema() -> Schema {
        schema!(enum {
            Included(%T::schema()),
            Excluded(%T::schema()),
            Unbounded(()),
        })
    }
}

impl<'a, T: SerdeSchema> SerdeSchema for &'a T {
    fn schema() -> Schema {
        T::schema()
    }
}

impl<'a, T: SerdeSchema> SerdeSchema for &'a mut T {
    fn schema() -> Schema {
        T::schema()
    }
}

impl<'a, T: SerdeSchema + ToOwned> SerdeSchema for Cow<'a, T> {
    fn schema() -> Schema {
        T::schema()
    }
}

// TODO: unfortunately, there are more

impl SerdeSchema for Schema {
    fn schema() -> Schema {
        schema!(enum {
            Scalar(enum {
                U8(()),
                U16(()),
                U32(()),
                U64(()),
                U128(()),
                I8(()),
                I16(()),
                I32(()),
                I64(()),
                I128(()),
                F32(()),
                F64(()),
                Char(()),
                Bool(()),
            }),
            CharString(()),
            ByteString(()),
            Option(recurse(1)),
            Seq({
                (len: ?(u64)),
                (inner: recurse(2)),
            }),
            Tuple([_; recurse(2)]),
            Struct([_; {
                (name: str),
                (inner: recurse(3)),
            }]),
            Enum([_; {
                (name: str),
                (inner: recurse(3)),
            }]),
            Recurse(recurse(1)),
        })
    }
}
