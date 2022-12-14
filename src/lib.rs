
pub mod schema;
pub mod serde_schema;
pub mod encoder;

#[test]
fn test() -> std::io::Result<()> {
    use schema::*;
    use encoder::*;

    let schema = schema!({
        (name: str),
        (arm_lengths: [2; f32]),
    });

    let buf = Encoder::new(Vec::<u8>::new(), &schema)
        .begin_struct()?
            .begin_field("name")?.encode_str("Reed")?
            .begin_field("arm_lengths")?.begin_seq(2)?
                .begin_elem()?.encode_f32(3.14)?
                .begin_elem()?.encode_f32(4.97)?
            .finish()?
        .finish()?;
    println!("{:?}", buf);


    let schema = schema!(enum {
        Leaf(i32),
        Branch({
            (a: recurse(2)),
            (b: recurse(2))
        }),
    });

    let buf = Encoder::new(Vec::<u8>::new(), &schema)
        .begin_enum(1, "Branch")?.begin_struct()?
            .begin_field("a")?.begin_enum(0, "Leaf")?.encode_i32(7)?
            .begin_field("b")?.begin_enum(0, "Leaf")?.encode_i32(5)?
        .finish()?;
    println!("{:?}", buf);

    Ok(())
}
