
use binschema::*;
use std::{
    fmt::Debug,
    collections::HashMap,
};
use serde::{
    Serialize,
    Deserialize,
};


#[cfg(test)]
fn round_trip_test<T>(val: T)
where
    T: Debug + PartialEq + Serialize + for<'d> Deserialize<'d> + KnownSchema,
{
    println!("{:#?}", val);
    println!("{:#?}", Test1::schema());

    // prep
    let schema = T::schema();
    let mut coder_alloc = CoderStateAlloc::new();
    let mut buf = Vec::new();

    // serialize
    let mut coder = CoderState::new(&schema, coder_alloc);
    let mut encoder = Encoder::new(&mut coder, &mut buf);
    val.serialize(&mut encoder).unwrap();
    coder.is_finished_or_err().unwrap();
    coder_alloc = coder.into_alloc();

    println!("{:?}", buf);

    // deserialize
    let mut coder = CoderState::new(&schema, coder_alloc);
    let mut read = buf.as_slice();
    let mut decoder = Decoder::new(&mut coder, &mut read);
    let val2 = T::deserialize(&mut decoder).unwrap();
    coder.is_finished_or_err().unwrap();
    coder_alloc = coder.into_alloc();

    println!("{:#?}", val2);
    assert_eq!(val, val2);

    drop(coder_alloc);
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, KnownSchema)]
pub struct Test1 {
    foo: u32,
    bar: String,
    baz: [i16; 4],
}

#[test]
fn test_1() {
    round_trip_test(Test1 {
        foo: 500,
        bar: "hello world".into(),
        baz: [7; 4],
    });
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, KnownSchema)]
pub struct Test2Outer {
    first: Test2Inner,
    second: Test2Inner,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, KnownSchema)]
pub enum Test2Inner {
    Foo(u32),
    Bar {
        a: HashMap<String, String>,
        b: (i32, f32),
    },
}

#[test]
fn test_2() {
    round_trip_test(Test2Outer {
        first: Test2Inner::Foo(4),
        second: {
            let mut hmap = HashMap::new();
            hmap.insert("foo_key".into(), "foo_val".into());
            hmap.insert("bar_key".into(), "bar_val".into());
            Test2Inner::Bar {
                a: hmap,
                b: (42, 3.14),
            }
        },
    });
}

#[derive(Debug, Clone, Serialize, Deserialize, KnownSchema)]
pub enum BinaryTree {
    Branch {
        value: u32,
        #[schema(recurse = 1)]
        left: Box<BinaryTree>,
        #[schema(recurse = 1)]
        right: Box<BinaryTree>,
    },
    Leaf(u32),
}

#[test]
fn binary_tree_test() {
    let binary_tree = BinaryTree::Branch {
        value: 5,
        left: Box::new(BinaryTree::Leaf(2)),
        right: Box::new(BinaryTree::Branch {
            value: 10,
            left: Box::new(BinaryTree::Leaf(7)),
            right: Box::new(BinaryTree::Leaf(20)),
        }),
    };

    println!("{:#?}", binary_tree);
    println!("{:#?}", BinaryTree::schema());
}
