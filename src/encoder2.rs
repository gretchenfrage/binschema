
use crate::{
    schema::*,
    error::*,
    var_len::*,
};
use std::io::{
    Result,
    Write,
    Read,
};


pub struct Encoder<'a, W> {
    stack: Vec<StackFrame<'a>>,
    write: W,
}

pub struct Decoder<'a, R> {
    stack: Vec<StackFrame<'a>>,
    read: R,
}

enum StackFrame<'a> {
    Value(&'a Schema),
    Seq {
        inner: &'a Schema,
        len: usize,
        next: usize,
    },
    Tuple {
        inner: &'a [Schema],
        next: usize,
    },
    Struct {
        inner: &'a [StructSchemaField],
        next: usize,
    },
}

macro_rules! validate {
    ($s:expr, $p:pat => $v:expr)=>{
        match $s.stack.pop() {
            Some(StackFrame::Value($p)) => $v,
            Some(top) => {
                let e = match &top {
                    &StackFrame::Value(need) => error!(
                        concat!(
                            "schema non-comformance, need {:?}, got ",
                            stringify!($p),
                        ),
                        need,
                    ),
                    &StackFrame::Seq { .. } => error!(
                        "invalid api usage, got value, but need seq item"
                    ),
                    &StackFrame::Tuple { .. } => error!(
                        "invalid api usage, got value, but need tuple item"
                    ),
                    &StackFrame::Struct { .. } => error!(
                        "invalid api usage, got value, but need struct field"
                    ),
                };
                $s.stack.push(top);
                return Err(error!(
                    "schema"
                ));
            },
            None => return Err(error!(
                "invalid api usage, got value, but finished",
            )),
        }
    };
}

impl<'a, W: Write> Encoder<'a, W> {
    pub fn encode_i32(&mut self, n: i32) -> Result<()> {
        validate!(self, &Schema::Scalar(ScalarType::I32) => ());
        write_var_len_sint(&mut self.write, n as i128)
    }

    pub fn encode_none(&mut self) -> Result<()> {
        let inner = validate!(self, &Schema::Option(_) => ());
        self.write.write_all(&[0])?;
        Ok(())
    }

    pub fn begin_some(&mut self) -> Result<()> {
        let inner = validate!(self, &Schema::Option(ref inner) => &**inner);
        self.write.write_all(&[1])?;
        self.stack.push(StackFrame::Value(inner));
        Ok(())
    }

    pub fn begin_tuple(&mut self) -> Result<()> {
        let inner = validate!(self, &Schema::Tuple(ref inner) => &**inner);
        self.stack.push(StackFrame::Tuple { inner, next: 0 });
    }
    
    pub fn begin_tuple_elem(&mut self) -> Result<()> {
        if let Some(top) = self.stack.last() {
            
        }
    }
}
