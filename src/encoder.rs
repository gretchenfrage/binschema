
use crate::schema::*;
use std::io::{
    Write,
    Error,
    ErrorKind,
    Result,
};

macro_rules! error {
    ($($e:tt)*)=>{
        Error::new(
            ErrorKind::Other,
            format!($($e)*),
        )
    };
}

macro_rules! ensure {
    ($c:expr, $($e:tt)*)=>{
        if !$c {
            return Err(error!($($e)*));
        }
    };
}


pub trait Outer<W> {
    type Encoder;

    fn encoder(self, write: W) -> Self::Encoder;
}


pub struct EncoderState<'a, O> {
    schema: &'a Schema,
    outer: O,
}

impl<'a, W, O> Outer<W> for EncoderState<'a, O> {
    type Encoder = Encoder<'a, W, O>;

    fn encoder(self, write: W) -> Self::Encoder {
        Encoder {
            state: self,
            write,
        }
    }
}

pub struct Encoder<'a, W, O> {
    state: EncoderState<'a, O>,
    write: W,
}

macro_rules! encode_le_bytes {
    ($($m:ident($t:ty),)*)=>{$(
        pub fn $m(mut self, n: $t) -> Result<O::Encoder> {
            self.validate(schema!(i32))?;
            self.write.write_all(&n.to_le_bytes())?;
            Ok(self.state.outer.encoder(self.write))
        }
    )*};
}

impl<'a, W: Write, O: Outer<W>> Encoder<'a, W, O> {
    fn validate(&self, got: Schema) -> Result<()> {
        ensure!(
            self.state.schema == &got,
            "schema non-comformance, need {:?}, got {:?}",
            self.state.schema,
            got,
        );
        Ok(())
    }

    encode_le_bytes!(
        encode_u8(u8),
        encode_u16(u16),
        encode_u32(u32),
        encode_u64(u64),
        encode_u128(u128),
        encode_i8(i8),
        encode_i16(i16),
        encode_i32(i32),
        encode_i64(i64),
        encode_i128(i128),
        encode_f32(f32),
        encode_f64(f64),
    );

    pub fn encode_char(mut self, c: char) -> Result<O::Encoder> {
        self.validate(schema!(char))?;
        self.write.write_all(&(c as u32).to_le_bytes())?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_bool(mut self, c: bool) -> Result<O::Encoder> {
        self.validate(schema!(bool))?;
        self.write.write_all(&[c as u8])?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_str(mut self, s: &str) -> Result<O::Encoder> {
        self.validate(schema!(str))?;
        self.write.write_all(s.as_bytes())?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_bytes(mut self, b: &[u8]) -> Result<O::Encoder> {
        self.validate(schema!(bytes))?;
        self.write.write_all(b)?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_unit(self) -> Result<O::Encoder> {
        self.validate(schema!(()))?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_none(mut self) -> Result<O::Encoder> {
        ensure!(
            matches!(self.state.schema, &Schema::Option(_)),
            "schema non-comformance, need {:?}, got Option",
            self.state.schema,
        );
        self.write.write_all(&[0])?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn start_some(mut self) -> Result<Encoder<'a, W, OptionContextLayer<'a, EncoderState<'a, O>>>> {
        match self.state.schema {
            &Schema::Option(ref inner_schema) => {
                self.write.write_all(&[1])?;
                Ok(Encoder {
                    state: EncoderState {
                        schema: inner_schema,
                        outer: OptionContextLayer {
                            inner_schema,
                            outer: self.state,
                        },
                    },
                    write: self.write,
                })
            },
            need => Err(error!(
                "schema non-comformance, need {:?}, got Option",
                need,
            )),
        }
    }

    pub fn start_seq(mut self, len: usize) -> Result<SeqEncoder<'a, W, EncoderState<'a, O>>> {
        match self.state.schema {
            &Schema::Seq(SeqSchema {
                len: need_len,
                inner: ref inner_schema
            }) => {
                if let Some(need_len) = need_len {
                    ensure!(
                        need_len == len,
                        "schema non-comformance, need seq len {}, got {}",
                        need_len,
                        len,
                    );
                } else {
                    self.write.write_all(&(len as u64).to_le_bytes())?;
                }
                Ok(SeqEncoder {
                    state: SeqEncoderState {
                        len,
                        inner_schema,
                        outer: self.state,
                        count: 0,
                    },
                    write: self.write,
                })
            },
            need => Err(error!(
                "schema non-comformance, need {:?}, got Seq",
                need,
            )),
        }
    }

    pub fn start_tuple(self) -> Result<TupleEncoder<'a, W, EncoderState<'a, O>>> {
        match self.state.schema {
            &Schema::Tuple(ref inner_schemas) => Ok(TupleEncoder {
                state: TupleEncoderState {
                    remaining_inner_schemas: inner_schemas,
                    outer: self.state,
                },
                write: self.write,
            }),
            need => Err(error!(
                "schema non-comformance, need {:?}, got Tuple",
                need,
            )),
        }
    }

    pub fn start_struct(self) -> Result<StructEncoder<'a, W, EncoderState<'a, O>>> {
        match self.state.schema {
            &Schema::Struct(ref fields) => Ok(StructEncoder {
                state: StructEncoderState {
                    remaining_fields: fields,
                    outer: self.state,
                },
                write: self.write,
            }),
            need => Err(error!(
                "schema non-comformance, need {:?}, got Tuple",
                need,
            )),
        }
    }

    pub fn start_enum(
        mut self,
        variant_ord: usize,
        variant_name: &str,
    ) -> Result<Encoder<'a, W, EnumContextLayer<'a, EncoderState<'a, O>>>> {
        match self.state.schema {
            &Schema::Enum(ref variants) => {
                ensure!(
                    variant_ord < variants.len(),
                    "schema non-comformance, only {} variants, but got variant ordinal {}",
                    variants.len(),
                    variant_ord,
                );
                let &EnumSchemaVariant {
                    name: ref need_name,
                    inner: ref inner_schema,
                } = &variants[variant_ord];
                ensure!(
                    variant_name == need_name,
                    "schema non-comformance, variant is named {:?}, but got name {:?}",
                    need_name,
                    variant_name,
                );
                self.write.write_all(&(variant_ord as u64).to_le_bytes())?;
                Ok(Encoder {
                    state: EncoderState {
                        schema: inner_schema,
                        outer: EnumContextLayer {
                            variants,
                            outer: self.state,
                        },
                    },
                    write: self.write,
                })
            },
            need => Err(error!(
                "schema non-comformance, need {:?}, got Enum",
                need,
            )),
        }
    }
}

pub struct OptionContextLayer<'a, O> {
    inner_schema: &'a Schema,
    outer: O,
}

impl<'a, W, O: Outer<W>> Outer<W> for OptionContextLayer<'a, O> {
    type Encoder = <O as Outer<W>>::Encoder;

    fn encoder(self, write: W) -> Self::Encoder {
        self.outer.encoder(write)
    }
}


pub struct SeqEncoderState<'a, O> {
    len: usize,
    inner_schema: &'a Schema,
    outer: O,
    count: usize,
}

impl<'a, W, O> Outer<W> for SeqEncoderState<'a, O> {
    type Encoder = SeqEncoder<'a, W, O>;

    fn encoder(self, write: W) -> Self::Encoder {
        SeqEncoder {
            state: self,
            write,
        }
    }
}

pub struct SeqEncoder<'a, W, O> {
    state: SeqEncoderState<'a, O>,
    write: W,
}

impl<'a, W: Write, O: Outer<W>> SeqEncoder<'a, W, O> {
    pub fn start_elem(mut self) -> Result<Encoder<'a, W, SeqEncoderState<'a, O>>> {
        ensure!(
            self.state.count < self.state.len,
            "too many start_elem calls, promised exactly {}",
            self.state.len,
        );
        self.state.count += 1;

        Ok(Encoder {
            state: EncoderState {
                schema: self.state.inner_schema,
                outer: self.state,
            },
            write: self.write,
        })
    }

    pub fn finish(self) -> Result<O::Encoder> {
        ensure!(
            self.state.count == self.state.len,
            "not enough start_elem calls, promised exactly {} but provided {}",
            self.state.len,
            self.state.count,
        );
        Ok(self.state.outer.encoder(self.write))
    }
}

pub struct TupleEncoderState<'a, O> {
    remaining_inner_schemas: &'a [Schema],
    outer: O,
}

impl<'a, W, O> Outer<W> for TupleEncoderState<'a, O> {
    type Encoder = TupleEncoder<'a, W, O>;

    fn encoder(self, write: W) -> Self::Encoder {
        TupleEncoder {
            state: self,
            write,
        }
    }
}

pub struct TupleEncoder<'a, W, O> {
    state: TupleEncoderState<'a, O>,
    write: W,
}

impl<'a, W: Write, O: Outer<W>> TupleEncoder<'a, W, O> {
    pub fn start_elem(mut self) -> Result<Encoder<'a, W, TupleEncoderState<'a, O>>> {
        if let Some((first, rest)) = self.state.remaining_inner_schemas.split_first() {
            self.state.remaining_inner_schemas = rest;

            Ok(Encoder {
                state: EncoderState {
                    schema: first,
                    outer: self.state,
                },
                write: self.write,
            })
        } else {
            Err(error!(
                "too many Tuple start_elem calls"
            ))
        }
    }

    pub fn finish(self) -> Result<O::Encoder> {
        ensure!(
            self.state.remaining_inner_schemas.is_empty(),
            "not enough Tuple start_elem calls, expected additional elements: {:?}",
            self.state.remaining_inner_schemas,
        );
        Ok(self.state.outer.encoder(self.write))
    }
}

pub struct StructEncoderState<'a, O> {
    remaining_fields: &'a [StructSchemaField],
    outer: O,
}

impl<'a, W, O> Outer<W> for StructEncoderState<'a, O> {
    type Encoder = StructEncoder<'a, W, O>;

    fn encoder(self, write: W) -> Self::Encoder {
        StructEncoder {
            state: self,
            write,
        }
    }
}

pub struct StructEncoder<'a, W, O> {
    state: StructEncoderState<'a, O>,
    write: W,
}

impl<'a, W: Write, O: Outer<W>> StructEncoder<'a, W, O> {
    pub fn start_field(mut self, name: &str) -> Result<Encoder<'a, W, StructEncoderState<'a, O>>> {
        if let Some((first, rest)) = self.state.remaining_fields.split_first() {
            let &StructSchemaField {
                name: ref need_name,
                inner: ref inner_schema,
            } = first;

            ensure!(
                need_name == name,
                "schema non-comformance, need field {:?}, got field {:?}",
                need_name,
                name,
            );

            self.state.remaining_fields = rest;

            Ok(Encoder {
                state: EncoderState {
                    schema: inner_schema,
                    outer: self.state,
                },
                write: self.write,
            })
        } else {
            Err(error!(
                "too many start_field calls"
            ))
        }
    }

    pub fn finish(self) -> Result<O::Encoder> {
        ensure!(
            self.state.remaining_fields.is_empty(),
            "not enough start_field calls, expected additional fields: {:?}",
            self.state.remaining_fields,
        );
        Ok(self.state.outer.encoder(self.write))
    }
}

pub struct EnumContextLayer<'a, O> {
    variants: &'a [EnumSchemaVariant],
    outer: O,
}

impl<'a, W, O: Outer<W>> Outer<W> for EnumContextLayer<'a, O> {
    type Encoder = <O as Outer<W>>::Encoder;

    fn encoder(self, write: W) -> Self::Encoder {
        self.outer.encoder(write)
    }
}
