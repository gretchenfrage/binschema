
pub mod schema;
pub mod value;

mod error;
mod serde_schema;
mod var_len;
mod coder;
mod encoder;
mod decoder;

pub use crate::{
    schema::{
        Schema,
        ScalarType,
        SeqSchema,
        StructSchemaField,
        EnumSchemaVariant,
    },
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
