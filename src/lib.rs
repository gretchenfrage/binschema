//! This serialization system is designed around the idea that that a _schema_,
//! a specification for what values are permitted and how they're encoded as
//! raw bytes, is a data structure that can be manipulated programmatically
//! at runtime and itself serialized. This can be used to achieve
//! bincode-levels of efficiency, protobufs levels of validation, and JSON
//! levels of easy debugging. For example, one could arrange a key/value store
//! such that the store contains, on-disk, the serialized schemas for the keys
//! and the values. Or, an RPC protocol could be designed such that, upon
//! initialization, the server sends down its list of endpoints and the
//! serialized schemas for their parameters and return types.
//!
//! Typical usage pattern:
//!
//! - create `CoderStateAlloc`
//! - to encode (serialize) a value:
//!     1. combine `&Schema` and `CoderStateAlloc` into `CoderState`
//!     2. combine `&mut CoderState` and `&mut W` where `W: Write` into `Encoder`
//!     3. pass `Encoder` and `&`value into procedure for encoding value
//!     4. on `CoderState`, call `.is_finished_or_err()?` to guarantee that
//!        valid schema-comformant data was fully written to `W`
//!     5. convert `CoderState` back into `CoderStateAlloc` so it can be reused
//! - to decode (deserialize) a value:
//!     1. combine `&Schema` and `CoderStateAlloc` into `CoderState`
//!     2. combine `&mut CoderState` and `&mut R` where `R: Read` into `Decoder`
//!     3. pass `Decoder` into procedure for decoding value
//!     4. on `CoderState`, call `.is_finished_or_err()?` to guarantee that
//!        valid schema-comformant data was fully read from `R`, and no more
//!     5. convert `CoderState` back into `CoderStateAlloc` so it can be reused
//!
//! The data model supports:
//!
//! - `u8` through `u128`, `i8` through `i128`(32 bits and above are encoded
//!    variable length)
//! - `f32` and `f64`, `char`, `bool`
//! - utf8 string, byte string
//! - option
//! - fixed length array, variable length array
//! - tuple (just values back-to-back)
//! - struct (just values back-to-back, but at schema-time the fields have 
//!   names)
//! - enum, as in rust-style enum, as in tagged union, as in "one of"
//! - recursing up in the schema, so as to support recursive schema types like
//!   trees


pub mod schema;
pub mod value;

mod do_if_err;
mod error;
mod serde_schema;
mod var_len;
mod coder;
mod encoder;
mod decoder;

pub use crate::schema::Schema;

pub use crate::{
    coder::{
        coder::CoderState,
        coder_alloc::CoderStateAlloc,
    },
    encoder::Encoder,
    decoder::Decoder,
};


#[test]
fn test() -> std::io::Result<()> {
    use schema::*;
    use encoder::*;

    let schema = schema!({
        (name: str),
        (arm_lengths: [2; f32]),
    });
    println!("{:#?}", schema);

    let mut buf = Vec::<u8>::new();
    let mut encoder_state = EncoderState::new(&schema, &mut buf, Default::default());
    encoder_state.encoder()
        .begin_struct()?
            .begin_struct_field("name")?.encode_str("Reed")?
            .begin_struct_field("arm_lengths")?.begin_seq(2)?
                .begin_seq_elem()?.encode_f32(3.14)?
                .begin_seq_elem()?.encode_f32(4.97)?
            .finish_seq()?
        .finish_struct()?;
    encoder_state.is_finished_or_err()?;
    let encoder_state = encoder_state.into_alloc();
    println!("{:?}", buf);

    let schema = schema!(enum {
        Leaf(i32),
        Branch({
            (n: i32),
            (a: recurse(2)),
            (b: recurse(2))
        }),
    });
    println!("{:#?}", schema);

    let mut buf = Vec::<u8>::new();
    let mut encoder_state = EncoderState::new(&schema, &mut buf, encoder_state);
    encoder_state.encoder()
        .begin_enum(1, "Branch")?.begin_struct()?
            .begin_struct_field("n")?.encode_i32(6)?
            .begin_struct_field("a")?.begin_enum(0, "Leaf")?.encode_i32(3)?
            .begin_struct_field("b")?.begin_enum(0, "Leaf")?.encode_i32(9)?
        .finish_struct()?;
    println!("{:?}", buf);


    Ok(())
}
