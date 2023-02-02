//! Encoder.


use crate::{
    error::*,
    schema::*,
    var_len::{
        write_var_len_uint,
        write_var_len_sint,
        write_ord,
    },
};
use std::{
    io::{
        Write,
        Result,
    },
    mem::forget,
    fmt::{self, Formatter, Debug},
};

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
        stack.clear();
        let ptr = stack.as_mut_ptr() as *mut ();
        let capacity = stack.capacity();
        forget(stack);
        EncoderStateAlloc { ptr, capacity }
    }



    fn into_stack<'a>(self) -> Vec<StackFrame<'a>> {
        unsafe {
            let stack = Vec::from_raw_parts(
                self.ptr as *mut StackFrame<'a>,
                0,
                self.capacity,
            );
            forget(self);
            stack
        }
    }
}

impl Drop for EncoderStateAlloc {
    fn drop(&mut self) {
        unsafe {
            drop(Vec::from_raw_parts(
                self.ptr as *mut StackFrame<'_>,
                0,
                self.capacity,
            ));
        }
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
    AutoFinishInProg,
    NeedEncode,
    SeqInProg {
        declared_len: usize,
        encoded_len: usize,
    },
    TupleInProg {
        next_need: usize,
    },
    StructInProg {
        next_need: usize,
    },
}

impl<'a, W> EncoderState<'a, W> {
    pub fn new(
        schema: &'a Schema,
        write: W,
        alloc: EncoderStateAlloc,
    ) -> Self {
        let mut stack = alloc.into_stack();
        stack.push(StackFrame {
            schema,
            api_state: ApiState::NeedEncode,
        });
        EncoderState { stack, write }
    }

    pub fn encoder<'b>(&'b mut self) -> Encoder<'b, 'a, W> {
        Encoder(self)
    }

    pub fn is_finished(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn is_finished_or_err(&self) -> Result<()> {
        if self.is_finished() {
            Ok(())
        } else {
            Err(error!("API usage error, didn't finish encoding"))
        }
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

macro_rules! validate_top {
    ($self:ident, |$top:ident| $opt_ret:expr, $got:expr)=>{
        match $self.0.stack.iter_mut().rev().next() {
            None => bail!("API usage error, usage of finished encoder"),
            Some($top) => match $opt_ret {
                Some(ret) => ret,
                None => match &$top.api_state {
                    &ApiState::AutoFinishInProg => unreachable!(),
                    &ApiState::NeedEncode => bail!(
                        "schema non-comformance, need encode {:?}, got {}",
                        $top.schema,
                        $got,
                    ),
                    &ApiState::SeqInProg { .. } => bail!(
                        "API usage error, need seq elem/finish, got {}",
                        $got,
                    ),
                    &ApiState::TupleInProg { .. } => bail!(
                        "API usage error, need tuple elem/finish, got {}",
                        $got,
                    ),
                    &ApiState::StructInProg { .. } => bail!(
                        "API usage error, need struct field/finish, got {}",
                        $got,
                    ),
                },
            }
        }
    };
}

macro_rules! validate_top_matches {
    ($self:ident, $top:pat => $ret:expr, $need:expr)=>{
        validate_top!(
            $self,
            |top| match top {
                $top => Some($ret),
                _ => None,
            },
            $need
        )
    };
}

macro_rules! validate_need_encode_eq {
    ($self:ident, $got:expr)=>{
        validate_top!(
            $self,
            |s| if
                    matches!(&s.api_state, &ApiState::NeedEncode)
                    && s.schema == &$got
                {
                    Some(())
                } else {
                    None
                },
            format_args!("encode {:?}", $got)
        )
    };
}

macro_rules! validate_need_encode_matches {
    ($self:ident, $pat:pat => $ret:expr, $got:expr)=>{
        validate_top_matches!(
            $self,
            &mut StackFrame {
                schema: $pat,
                api_state: ApiState::NeedEncode,
            } => $ret,
            $got
        )
    };
}

macro_rules! match_or_unreachable {
    ($expr:expr, $pat:pat => $ret:expr)=>{
        match $expr {
            $pat => $ret,
            _ => unreachable!(),
        }
    };
}

macro_rules! encode_le_bytes {
    ($($m:ident($t:ident),)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<&mut Self> {
            validate_need_encode_eq!(self, schema!($t));
            self.write(&n.to_le_bytes())?;
            self.pop();
            Ok(self)
        }
    )*};
}

macro_rules! encode_var_len_uint {
    ($($m:ident($t:ident),)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<&mut Self> {
            validate_need_encode_eq!(self, schema!($t));
            write_var_len_uint(&mut self.0.write, n as u128)?;
            self.pop();
            Ok(self)
        }
    )*};
}

macro_rules! encode_var_len_sint {
    ($($m:ident($t:ident),)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<&mut Self> {
            validate_need_encode_eq!(self, schema!($t));
            write_var_len_sint(&mut self.0.write, n as i128)?;
            self.pop();
            Ok(self)
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

    fn push_need_encode(&mut self, mut schema: &'a Schema) -> Result<()> {
        let mut i = self.0.stack.len();
        while let &Schema::Recurse(n) = schema {
            ensure!(
                n > 0,
                "invalid schema: recurse of level 0"
            );
            i = i
                .checked_sub(n)
                .ok_or_else(|| error!("invalid schema: recurse past base of stack"))?;
            schema = self.0.stack[i].schema;
        }
        self.0.stack.push(StackFrame {
            schema,
            api_state: ApiState::NeedEncode,
        });
        Ok(())
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
        encode_i8(i8),
        encode_i16(i16),
        encode_f32(f32),
        encode_f64(f64),
    );

    encode_var_len_uint!(
        encode_u32(u32),
        encode_u64(u64),
        encode_u128(u128),
    );

    encode_var_len_sint!(
        encode_i32(i32),
        encode_i64(i64),
        encode_i128(i128),
    );

    pub fn encode_char(&mut self, c: char) -> Result<&mut Self> {
        validate_need_encode_eq!(self, schema!(char));
        self.write(&(c as u32).to_le_bytes())?;
        self.pop();
        Ok(self)
    }

    pub fn encode_bool(&mut self, b: bool) -> Result<&mut Self> {
        validate_need_encode_eq!(self, schema!(bool));
        self.write(&[b as u8])?;
        self.pop();
        Ok(self)
    }

    pub fn encode_unit(&mut self) -> Result<&mut Self> {
        validate_need_encode_eq!(self, schema!(()));
        self.pop();
        Ok(self)
    }

    pub fn encode_str(&mut self, s: &str) -> Result<&mut Self> {
        validate_need_encode_eq!(self, schema!(str));
        write_var_len_uint(&mut self.0.write, s.len() as u128)?;
        self.write(s.as_bytes())?;
        self.pop();
        Ok(self)
    }

    pub fn encode_bytes(&mut self ,s: &[u8]) -> Result<&mut Self> {
        validate_need_encode_eq!(self, schema!(bytes));
        write_var_len_uint(&mut self.0.write, s.len() as u128)?;
        self.write(s)?;
        self.pop();
        Ok(self)
    }

    /// Completely encode a None value for an Option schema.
    pub fn encode_none(&mut self) -> Result<&mut Self> {
        validate_need_encode_matches!(
            self,
            &Schema::Option(_) => (),
            "encode option"
        );
        self.write(&[0])?;
        self.pop();
        Ok(self)
    }

    /// Begin encoding a Some value for an Option schema. This should be
    /// followed by encoding the inner value, after which the Option will
    /// auto-finish.
    pub fn begin_some(&mut self) -> Result<&mut Self> {
        let inner =
            validate_need_encode_matches!(
                self,
                &Schema::Option(ref inner) => inner,
                "encode option"
            );
        self.write(&[1])?;
        self.top().api_state = ApiState::AutoFinishInProg;
        self.push_need_encode(inner)?;
        Ok(self)
    }

    /// Begin encoding a seq. This should be followed by encoding the elements
    /// with `begin_seq_elem` followed by a call to `finish_seq`.
    pub fn begin_seq(&mut self, len: usize) -> Result<&mut Self> {
        let need_len =
            validate_need_encode_matches!(
                self,
                &Schema::Seq(SeqSchema { len, .. }) => len,
                "encode seq"
            );
        if let Some(need_len) = need_len {
            ensure!(
                need_len == len,
                "schema non-comformance, need seq len {}, got seq len {}",
                need_len,
                len
            );
        } else {
            write_var_len_uint(&mut self.0.write, len as u128)?;
        }
        self.top().api_state =
            ApiState::SeqInProg {
                declared_len: len,
                encoded_len: 0,
            };
        Ok(self)
    }

    /// Begin encoding an element in a seq. This should be followed by encoding
    /// the inner value. See `begin_seq`,
    pub fn begin_seq_elem(&mut self) -> Result<&mut Self> {
        let (schema, declared_len, encoded_len) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::SeqInProg {
                        declared_len,
                        ref mut encoded_len,
                    }
                } => (schema, declared_len, encoded_len),
                "seq elem"
            );
        ensure!(
            *encoded_len + 1 <= declared_len,
            "API usage error, begin seq elem at idx {}, but that is seq's declared len",
            encoded_len
        );
        *encoded_len += 1;
        self
            .push_need_encode(match_or_unreachable!(
                schema,
                &Schema::Seq(SeqSchema { ref inner, .. }) => &**inner
            ))?;
        Ok(self)
    }

    /// Finish encoding a seq. See `begin_seq`.
    pub fn finish_seq(&mut self) -> Result<&mut Self> {
        let (declared_len, encoded_len) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    api_state: ApiState::SeqInProg {
                        declared_len,
                        encoded_len,
                    },
                    ..
                } => (declared_len, encoded_len),
                "seq finish"
            );
        ensure!(
            encoded_len == declared_len,
            "API usage error, finish seq of declared len {}, but only encoded {} elems",
            declared_len,
            encoded_len
        );
        self.pop();
        Ok(self)
    }

    /// Begin encoding a tuple. This should be followed by encoding the
    /// elements with `begin_tuple_elem` followed by a call to `finish_tuple`.
    pub fn begin_tuple(&mut self) -> Result<&mut Self> {
        validate_need_encode_matches!(
            self,
            &Schema::Tuple(_) => (),
            "encode tuple"
        );
        self.top().api_state =
            ApiState::TupleInProg {
                next_need: 0,
            };
        Ok(self)
    }

    /// Begin encoding an element in a tuple. This should be followed by
    /// encoding the inner value. See `begin_tuple`,
    pub fn begin_tuple_elem(&mut self) -> Result<&mut Self> {
        let (schema, next_need) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::TupleInProg {
                        ref mut next_need,
                    },
                } => (schema, next_need),
                "tuple elem"
            );
        let inner_schema =
            match_or_unreachable!(
                schema,
                &Schema::Tuple(ref inners) => inners
            )
            .get(*next_need)
            .ok_or_else(|| error!(
                "schema non-comformance, begin tuple elem at idx {}, but that is the tuple's len",
                *next_need,
            ))?;
        *next_need += 1;
        self.push_need_encode(inner_schema)?;
        Ok(self)
    }

    /// Finish encoding a tuple. See `begin_tuple`.
    pub fn finish_tuple(&mut self) -> Result<&mut Self> {
        let (schema, next_need) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::TupleInProg {
                        next_need,
                    },
                } => (schema, next_need),
                "tuple finish"
            );
        let inners = 
            match_or_unreachable!(
                schema,
                &Schema::Tuple(ref inners) => inners
            );
        ensure!(
            inners.len() == next_need,
            "schema non-comformance, finish tuple of len {}, but only encoded {} elems",
            inners.len(),
            next_need,
        );
        self.pop();
        Ok(self)
    }

    /// Begin encoding a struct. This should be followed by encoding the
    /// fields with `begin_struct_field` followed by a call to `finish_struct`.
    pub fn begin_struct(&mut self) -> Result<&mut Self> {
        validate_need_encode_matches!(
            self,
            &Schema::Struct(_) => (),
            "encode struct"
        );
        self.top().api_state =
            ApiState::StructInProg {
                next_need: 0,
            };
        Ok(self)
    }

    /// Begin encoding a field in a struct. This should be followed by
    /// encoding the inner value. See `begin_struct`,
    pub fn begin_struct_field(&mut self, name: &str) -> Result<&mut Self> {
        let (schema, next_need) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::StructInProg {
                        ref mut next_need,
                    },
                } => (schema, next_need),
                "struct field"
            );
        let field =
            match_or_unreachable!(
                schema,
                &Schema::Struct(ref fields) => fields
            )
            .get(*next_need)
            .ok_or_else(|| error!(
                "schema non-comformance, begin struct field at idx {}, but that is the struct's len",
                *next_need,
            ))?;
        ensure!(
            &field.name == name,
            "schema non-comformance, need struct field {:?}, got struct field {:?}",
            field.name,
            name,
        );
        *next_need += 1;
        self.push_need_encode(&field.inner)?;
        Ok(self)
    }

    /// Finish encoding a struct. See `begin_struct`.
    pub fn finish_struct(&mut self) -> Result<&mut Self> {
        let (schema, next_need) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::StructInProg {
                        next_need,
                    },
                } => (schema, next_need),
                "struct finish"
            );
        let fields = 
            match_or_unreachable!(
                schema,
                &Schema::Struct(ref fields) => fields
            );
        ensure!(
            fields.len() == next_need,
            "schema non-comformance, finish struct of len {}, but only encoded {} elems",
            fields.len(),
            next_need,
        );
        self.pop();
        Ok(self)
    }

    /// Begin encoding a variant for an enum schema. This should be followed by
    /// encodnig the inner value, after which the enum will auto-finish.
    pub fn begin_enum(
        &mut self,
        variant_ord: usize,
        variant_name: &str,
    ) -> Result<&mut Self> {
        let variants =
            validate_need_encode_matches!(
                self,
                &Schema::Enum(ref variants) => variants,
                "encode enum"
            );
        let variant = variants
            .get(variant_ord)
            .ok_or_else(|| error!(
                "schema non-comformance, begin enum with variant ordinal {}, but enum only has {} variants",
                variant_ord,
                variants.len(),
            ))?;
        ensure!(
            variant_name == &variant.name,
            "schema non-comformance, begin enum with variant name {:?}, but variant at that ordinal has name {:?}",
            variant_name,
            variant.name,
        );
        write_ord(&mut self.0.write, variant_ord, variants.len())?;
        self.top().api_state = ApiState::AutoFinishInProg;
        self.push_need_encode(&variant.inner)?;
        Ok(self)
    }
}
