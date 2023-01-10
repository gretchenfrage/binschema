
use crate::schema::*;
use std::{
    io::{
        Write,
        Error,
        ErrorKind,
        Result,
    },
    mem::forget,
    fmt::{self, Formatter, Debug},
};

macro_rules! error {
    ($($e:tt)*)=>{
        Error::new(
            ErrorKind::Other,
            format!($($e)*),
        )
    };
}

macro_rules! bail {
    ($($e:tt)*)=>{
        return Err(error!($($e)*))
    };
}

macro_rules! ensure {
    ($c:expr, $($e:tt)*)=>{
        if !$c {
            bail!($($e)*);
        }
    };
}

#[derive(Debug)]
pub struct EncoderStateAlloc {
    ptr: *mut (),
    capacity: usize,
}

impl EncoderStateAlloc {
    pub fn new() -> Self {
        Self::from_stack(Vec::new())
    }

    fn from_stack(mut stack: Vec<StackFrame<'_>>) -> Self {
        let ptr = stack.as_mut_ptr() as *mut ();
        let capacity = stack.capacity();
        forget(stack);
        EncoderStateAlloc { ptr, capacity }
    }
}

impl Default for EncoderStateAlloc {
    fn default() -> Self {
        EncoderStateAlloc::new()
    }
}

pub struct EncoderState<'a, W> {
    stack: Vec<StackFrame<'a>>,
    write: W,
}

impl<'a, W> Debug for EncoderState<'a, W> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("EncoderState")
            .field("stack", &self.stack)
            .finish_non_exhaustive()
    }
} 

#[derive(Debug)]
struct StackFrame<'a> {
    schema: &'a Schema,
    api_state: ApiState,
}

#[derive(Debug)]
enum ApiState {
    Need,
    AutoFinishInProg,
    StructInProg {
        next_need: usize,
    },
    TupleInProg {
        next_need: usize,
    },
    SeqInProg {
        declared_len: usize,
        encoded_len: usize,
    },
}

impl<'a, W> EncoderState<'a, W> {
    pub fn new(
        schema: &'a Schema,
        write: W,
        alloc: EncoderStateAlloc,
    ) -> Self {
        let mut stack = unsafe {
            Vec::from_raw_parts(
                alloc.ptr as *mut StackFrame<'a>,
                0,
                alloc.capacity,
            )
        };
        stack.push(StackFrame {
            schema,
            api_state: ApiState::Need,
        });
        EncoderState { stack, write }
    }

    pub fn encoder<'b>(&'b mut self) -> Encoder<'b, 'a, W> {
        Encoder(self)
    }

    pub fn is_finished(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn into_alloc(mut self) -> EncoderStateAlloc {
        self.stack.clear();
        EncoderStateAlloc::from_stack(self.stack)
    }
}

impl<'a, W> From<EncoderState<'a, W>> for EncoderStateAlloc {
    fn from(state: EncoderState<'a, W>) -> Self {
        state.into_alloc()
    }
}

#[derive(Debug)]
pub struct Encoder<'b, 'a, W>(&'b mut EncoderState<'a, W>);

macro_rules! validate_encode {
    ($self:ident, |$need:ident| $cond:expr, $got:expr,)=>{
        match $self.0.stack.iter().rev().next() {
            None => bail!("API usage error, encode to finished encoder"),
            Some(&StackFrame {
                schema: $need,
                api_state: ApiState::Need,
            }) => match $cond {
                Some(ret) => ret,
                None => bail!(
                    "schema non-comformance, need {:?}, got {}",
                    $need,
                    $got,
                ),
            }
            Some(&StackFrame { ref api_state, .. }) => bail!(
                "API usage error, need {}, got {}",
                match api_state {
                    &ApiState::Need => unreachable!(),
                    &ApiState::AutoFinishInProg => unreachable!(),
                    &ApiState::StructInProg { .. } => "struct field",
                    &ApiState::TupleInProg { .. } => "tuple elem",
                    &ApiState::SeqInProg { .. } => "seq elem",
                },
                stringify!($got),
            ),
        }
    };
}

macro_rules! validate_encode_eq {
    ($self:ident, $got:expr)=>{
        validate_encode!(
            $self,
            |s| if s == &$got { Some(()) } else { None },
            format_args!("{:?}", $got),
        )
    };
}

macro_rules! validate_encode_matches {
    ($self:ident, $got:pat => $ret:expr)=>{
        validate_encode!(
            $self,
            |s| match s {
                $got => Some($ret),
                _ => None,
            },
            stringify!($got),
        )
    };
}

macro_rules! encode_le_bytes {
    ($($m:ident($t:ident),)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<()> {
            validate_encode_eq!(self, schema!($t));
            self.write(&n.to_le_bytes())?;
            self.pop();
            Ok(())
        }
    )*};
}

impl<'b, 'a, W: Write> Encoder<'b, 'a, W> {
    fn write(&mut self, b: &[u8]) -> Result<()> {
        self.0.write.write_all(b)
    }

    fn top(&mut self) -> &mut StackFrame<'a> {
        let i = self.0.stack.len() - 1;
        &mut self.0.stack[i]
    }

    fn pop(&mut self) {
        self.0.stack.pop().unwrap();
        while matches!(
            self.0.stack.iter().rev().next(),
            Some(&StackFrame { api_state: ApiState::AutoFinishInProg, .. })
        ) {
            self.0.stack.pop().unwrap();
        }
    }


    encode_le_bytes!(
        encode_u8(u8),
        encode_u16(u16),
        //encode_u32(u32),
        //encode_u64(u64),
        encode_i8(i8),
        encode_i16(i16),
        //encode_u128(u128),
        //encode_i32(i32),
        //encode_i64(i64),
        //encode_i128(i128),
        //encode_f32(f32),
        //encode_f64(f64),
        // TODO: big int encoding
    );

    pub fn encode_char(&mut self, c: char) -> Result<()> {
        validate_encode_eq!(self, schema!(char));
        self.write(&(c as u32).to_le_bytes())?;
        self.pop();
        Ok(())
    }

    pub fn encode_bool(&mut self, b: bool) -> Result<()> {
        validate_encode_eq!(self, schema!(bool));
        self.write(&[b as u8])?;
        self.pop();
        Ok(())
    }

    pub fn encode_unit(&mut self) -> Result<()> {
        validate_encode_eq!(self, schema!(()));
        self.pop();
        Ok(())
    }

    /*
    pub fn encode_str(mut self, s: &str) -> Result<O::Encoder> {
        self.recurse()?;
        self.validate(schema!(str))?;
        self.write.write_all(s.as_bytes())?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_bytes(mut self, b: &[u8]) -> Result<O::Encoder> {
        self.recurse()?;
        self.validate(schema!(bytes))?;
        self.write.write_all(b)?;
        Ok(self.state.outer.encoder(self.write))
    }
    */

    /// Completely encode a None value for an Option schema.
    pub fn encode_none(&mut self) -> Result<()> {
        validate_encode_matches!(self, &Schema::Option(_) => ());
        self.write(&[0])?;
        self.pop();
        Ok(())
    }

    /// Begin encoding a Some value for an Option schema. This should be
    /// followed by encoding the inner value, after which the Option will
    /// auto-finish.
    pub fn begin_some(&mut self) -> Result<()> {
        let inner =
            validate_encode_matches!(
                self,
                &Schema::Option(ref inner) => inner
            );
        self.write(&[1])?;
        self.top().api_state = ApiState::AutoFinishInProg;
        self.0.stack.push(StackFrame {
            schema: inner,
            api_state: ApiState::Need,
        });
        Ok(())
    }
}


/*
pub trait Outer<'a, W> {
    type Encoder;

    fn encoder(self, write: W) -> Self::Encoder;

    fn recurse_schema(&self, n: usize) -> Option<&'a Schema>;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SchemaBase;

impl<'a, W> Outer<'a, W> for SchemaBase {
    type Encoder = W;

    fn encoder(self, write: W) -> Self::Encoder {
        write
    }

    fn recurse_schema(&self, _: usize) -> Option<&'a Schema> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct EncoderState<'a, O> {
    schema: &'a Schema,
    outer: O,
}

impl<'a, W, O: Outer<'a, W>> Outer<'a, W> for EncoderState<'a, O> {
    type Encoder = <O as Outer<'a, W>>::Encoder;

    fn encoder(self, write: W) -> Self::Encoder {
        self.outer.encoder(write)
    }

    fn recurse_schema(&self, n: usize) -> Option<&'a Schema> {
        match n {
            0 => Some(self.schema),
            n => self.outer.recurse_schema(n - 1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Encoder<'a, W, O> {
    state: EncoderState<'a, O>,
    write: W,
}

macro_rules! encode_le_bytes {
    ($($m:ident($t:ident),)*)=>{$(
        pub fn $m(mut self, n: $t) -> Result<O::Encoder> {
            self.recurse()?;
            self.validate(schema!($t))?;
            self.write.write_all(&n.to_le_bytes())?;
            Ok(self.state.outer.encoder(self.write))
        }
    )*};
}

impl<'a, W> Encoder<'a, W, SchemaBase> {
    pub fn new(write: W, schema: &'a Schema) -> Self {
        Encoder {
            state: EncoderState {
                schema,
                outer: SchemaBase,
            },
            write,
        }
    }
}

impl<'a, W: Write, O: Outer<'a, W>> Encoder<'a, W, O> {
    fn recurse(&mut self) -> Result<()> {
        while let &Schema::Recurse(n) = self.state.schema {
            ensure!(
                n > 0,
                "schema problem: recurse level 0 would cause infinite loop",
            );
            self.state.schema = self.state
                .recurse_schema(n)
                .ok_or_else(|| error!(
                    "schema problem: recurse level {} goes beyond root of schema",
                    n,
                ))?;
        }
        Ok(())
    }

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
        self.recurse()?;
        self.validate(schema!(char))?;
        self.write.write_all(&(c as u32).to_le_bytes())?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_bool(mut self, c: bool) -> Result<O::Encoder> {
        self.recurse()?;
        self.validate(schema!(bool))?;
        self.write.write_all(&[c as u8])?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_str(mut self, s: &str) -> Result<O::Encoder> {
        self.recurse()?;
        self.validate(schema!(str))?;
        self.write.write_all(s.as_bytes())?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_bytes(mut self, b: &[u8]) -> Result<O::Encoder> {
        self.recurse()?;
        self.validate(schema!(bytes))?;
        self.write.write_all(b)?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_unit(mut self) -> Result<O::Encoder> {
        self.recurse()?;
        self.validate(schema!(()))?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn encode_none(mut self) -> Result<O::Encoder> {
        self.recurse()?;
        ensure!(
            matches!(self.state.schema, &Schema::Option(_)),
            "schema non-comformance, need {:?}, got Option",
            self.state.schema,
        );
        self.write.write_all(&[0])?;
        Ok(self.state.outer.encoder(self.write))
    }

    pub fn begin_some(mut self) -> Result<Encoder<'a, W, EncoderState<'a, O>>> {
        self.recurse()?;
        match self.state.schema {
            &Schema::Option(ref inner_schema) => {
                self.write.write_all(&[1])?;
                Ok(Encoder {
                    state: EncoderState {
                        schema: inner_schema,
                        outer: self.state,
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

    pub fn begin_seq(mut self, len: usize) -> Result<SeqEncoder<'a, W, EncoderState<'a, O>>> {
        self.recurse()?;
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

    pub fn begin_tuple(mut self) -> Result<TupleEncoder<'a, W, EncoderState<'a, O>>> {
        self.recurse()?;
        match self.state.schema {
            &Schema::Tuple(ref inner_schemas) => Ok(TupleEncoder {
                state: TupleEncoderState {
                    inner_schemas,
                    outer: self.state,
                    count: 0,
                },
                write: self.write,
            }),
            need => Err(error!(
                "schema non-comformance, need {:?}, got Tuple",
                need,
            )),
        }
    }

    pub fn begin_struct(mut self) -> Result<StructEncoder<'a, W, EncoderState<'a, O>>> {
        self.recurse()?;
        match self.state.schema {
            &Schema::Struct(ref fields) => Ok(StructEncoder {
                state: StructEncoderState {
                    fields,
                    outer: self.state,
                    count: 0,
                },
                write: self.write,
            }),
            need => Err(error!(
                "schema non-comformance, need {:?}, got Tuple",
                need,
            )),
        }
    }

    pub fn begin_enum(
        mut self,
        variant_ord: usize,
        variant_name: &str,
    ) -> Result<Encoder<'a, W, EncoderState<'a, O>>> {
        self.recurse()?;
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
                        outer: self.state,
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

#[derive(Debug, Clone)]
pub struct SeqEncoderState<'a, O> {
    len: usize,
    inner_schema: &'a Schema,
    outer: O,
    count: usize,
}

impl<'a, W, O: Outer<'a, W>> Outer<'a, W> for SeqEncoderState<'a, O> {
    type Encoder = SeqEncoder<'a, W, O>;

    fn encoder(self, write: W) -> Self::Encoder {
        SeqEncoder {
            state: self,
            write,
        }
    }

    fn recurse_schema(&self, n: usize) -> Option<&'a Schema> {
        self.outer.recurse_schema(n)
    }
}

#[derive(Debug, Clone)]
pub struct SeqEncoder<'a, W, O> {
    state: SeqEncoderState<'a, O>,
    write: W,
}

impl<'a, W: Write, O: Outer<'a, W>> SeqEncoder<'a, W, O> {
    pub fn begin_elem(mut self) -> Result<Encoder<'a, W, SeqEncoderState<'a, O>>> {
        ensure!(
            self.state.count < self.state.len,
            "too many SeqEncoder::begin_elem calls, promised exactly {}",
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
            "not enough SeqEncoder::begin_elem calls, promised exactly {} but provided {}",
            self.state.len,
            self.state.count,
        );
        Ok(self.state.outer.encoder(self.write))
    }
}

#[derive(Debug, Clone)]
pub struct TupleEncoderState<'a, O> {
    inner_schemas: &'a [Schema],
    outer: O,
    count: usize,
}

impl<'a, W, O: Outer<'a, W>> Outer<'a, W> for TupleEncoderState<'a, O> {
    type Encoder = TupleEncoder<'a, W, O>;

    fn encoder(self, write: W) -> Self::Encoder {
        TupleEncoder {
            state: self,
            write,
        }
    }

    fn recurse_schema(&self, n: usize) -> Option<&'a Schema> {
        self.outer.recurse_schema(n)
    }
}

#[derive(Debug, Clone)]
pub struct TupleEncoder<'a, W, O> {
    state: TupleEncoderState<'a, O>,
    write: W,
}

impl<'a, W: Write, O: Outer<'a, W>> TupleEncoder<'a, W, O> {
    pub fn begin_elem(mut self) -> Result<Encoder<'a, W, TupleEncoderState<'a, O>>> {
        ensure!(
            self.state.count < self.state.inner_schemas.len(),
            "too many TupleEncoder::begin_elem calls, no additional elements expected",
        );
        let inner_schema = &self.state.inner_schemas[self.state.count];
        self.state.count += 1;

        Ok(Encoder {
            state: EncoderState {
                schema: inner_schema,
                outer: self.state,
            },
            write: self.write,
        })
    }

    pub fn finish(self) -> Result<O::Encoder> {
        ensure!(
            self.state.count == self.state.inner_schemas.len(),
            "not enough Tuple::begin_elem calls, expected additional elements: {:?}",
            &self.state.inner_schemas[self.state.count..],
        );
        Ok(self.state.outer.encoder(self.write))
    }
}

#[derive(Debug, Clone)]
pub struct StructEncoderState<'a, O> {
    fields: &'a [StructSchemaField],
    outer: O,
    count: usize,
}

impl<'a, W, O: Outer<'a, W>> Outer<'a, W> for StructEncoderState<'a, O> {
    type Encoder = StructEncoder<'a, W, O>;

    fn encoder(self, write: W) -> Self::Encoder {
        StructEncoder {
            state: self,
            write,
        }
    }

    fn recurse_schema(&self, n: usize) -> Option<&'a Schema> {
        self.outer.recurse_schema(n)
    }
}

#[derive(Debug, Clone)]
pub struct StructEncoder<'a, W, O> {
    state: StructEncoderState<'a, O>,
    write: W,
}

impl<'a, W: Write, O: Outer<'a, W>> StructEncoder<'a, W, O> {
    pub fn begin_field(mut self, name: &str) -> Result<Encoder<'a, W, StructEncoderState<'a, O>>> {
        ensure!(
            self.state.count < self.state.fields.len(),
            "too many begin_field calls, no additional fields expected",
        );
        let &StructSchemaField {
            name: ref need_name,
            inner: ref inner_schema,
        } = &self.state.fields[self.state.count];
        ensure!(
            need_name == name,
            "schema non-comformance, need field {:?}, got field {:?}",
            need_name,
            name,
        );
        self.state.count += 1;

        Ok(Encoder {
            state: EncoderState {
                schema: inner_schema,
                outer: self.state,
            },
            write: self.write,
        })
    }

    pub fn finish(self) -> Result<O::Encoder> {
        ensure!(
            self.state.count == self.state.fields.len(),
            "not enough begin_field calls, expected additional fields: {:?}",
            &self.state.fields[self.state.count..],
        );
        Ok(self.state.outer.encoder(self.write))
    }
}
*/