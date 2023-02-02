
use crate::{
    error::{
        error,
        ensure,
        bail,
    },
    schema::{
        Schema,
        schema,
    },
    coder::coder_alloc::CoderStateAlloc,
};
use std::io::Result;


#[derive(Debug, Clone)]
pub struct CoderState<'a> {
    stack: Vec<StackFrame<'a>>,
}

#[derive(Debug, Clone)]
pub(super) struct StackFrame<'a> {
    schema: &'a Schema,
    api_state: ApiState,
}

#[derive(Debug, Clone)]
enum ApiState {
    /// Starting state. This element needs to be coded, and has not started
    /// being coded.
    Need,
    /// Some inner element is being coded, and finishing encoding the inner
    /// element is sufficient for this element to be considered finished
    /// encoding.
    AutoFinish,
    /// A sequence is being coded. The corresponding `schema` must be a
    /// `Schema::Seq`.
    Seq {
        /// Total number of elements needing to be coded.
        len: usize,
        /// Next element index would code.
        next: usize,
    },
    /// A tuple is being coded. The corresponding `schema` must be a
    /// `Schema::Tuple`.
    Tuple {
        /// Next element index would code.
        next: usize,
    },
    /// A struct is being coded. The corresponding `schema` must be a
    /// `schema::Struct`.
    Struct {
        /// Next field index would code.
        next: usize,
    },
    /// An enum is being coded, but the variant has not yet been coded. The
    /// corresponding `schema` must be a `schema::Enum`.
    Enum,
}

impl<'a> CoderState<'a> {
    pub fn new(
        schema: &'a Schema,
        alloc: CoderStateAlloc,
    ) -> Self {
        let mut stack = alloc.into_stack();
        stack.push(StackFrame {
            schema,
            api_state: ApiState::Need,
        });
        CoderState { stack }
    }

    pub fn is_finished(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn is_finished_or_err(&self) -> Result<()> {
        if self.is_finished() {
            Ok(())
        } else {
            Err(error!("API usage error, didn't finish coding"))
        }
    }

    pub fn into_alloc(mut self) -> CoderStateAlloc {
        self.stack.clear();
        CoderStateAlloc::from_stack(self.stack)
    }
}

macro_rules! validate_top {
    ($self:ident, |$top:ident| $opt_ret:expr, $got:expr)=>{
        match $self.stack.iter_mut().rev().next() {
            None => bail!("API usage error, usage of finished coder"),
            Some($top) => match $opt_ret {
                Some(ret) => ret,
                None => match &$top.api_state {
                    &ApiState::AutoFinish => unreachable!(),
                    &ApiState::Need => bail!(
                        "schema non-comformance, need {:?}, got {}",
                        $top.schema,
                        $got,
                    ),
                    &ApiState::Seq { .. } => bail!(
                        "API usage error, need seq elem/finish, got {}",
                        $got,
                    ),
                    &ApiState::Tuple { .. } => bail!(
                        "API usage error, need tuple elem/finish, got {}",
                        $got,
                    ),
                    &ApiState::Struct { .. } => bail!(
                        "API usage error, need struct field/finish, got {}",
                        $got,
                    ),
                    &ApiState::Enum => bail!(
                        "API usage error, need enum variant, got {}",
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

macro_rules! validate_need_eq {
    ($self:ident, $got:expr)=>{
        validate_top!(
            $self,
            |s| if
                    matches!(&s.api_state, &ApiState::Need)
                    && s.schema == &$got
                {
                    Some(())
                } else {
                    None
                },
            format_args!("code {:?}", $got)
        )
    };
}

macro_rules! validate_need_matches {
    ($self:ident, $pat:pat => $ret:expr, $got:expr)=>{
        validate_top_matches!(
            $self,
            &mut StackFrame {
                schema: $pat,
                api_state: ApiState::Need,
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

macro_rules! code_simple {
    ($($m:ident($($t:tt)*),)*)=>{$(
        pub fn $m(&mut self) -> Result<()> {
            validate_need_eq!(self, schema!($($t)*));
            self.pop();
            Ok(())
        }
    )*};
}

impl<'a> CoderState<'a> {
    fn top(&mut self) -> &mut StackFrame<'a> {
        let i = self.stack.len() - 1;
        &mut self.stack[i]
    }

    fn push_need(&mut self, mut schema: &'a Schema) -> Result<()> {
        let mut i = self.stack.len();
        while let &Schema::Recurse(n) = schema {
            ensure!(
                n > 0,
                "invalid schema: recurse of level 0"
            );
            i = i
                .checked_sub(n)
                .ok_or_else(|| error!("invalid schema: recurse past base of stack"))?;
            schema = self.stack[i].schema;
        }
        self.stack.push(StackFrame {
            schema,
            api_state: ApiState::Need,
        });
        Ok(())
    }

    fn pop(&mut self) {
        self.stack.pop().unwrap();
        while matches!(
            self.stack.iter().rev().next(),
            Some(&StackFrame { api_state: ApiState::AutoFinish, .. })
        ) {
            self.stack.pop().unwrap();
        }
    }

    code_simple!(
        code_u8(u8),
        code_u16(u16),
        code_u32(u32),
        code_u64(u64),
        code_u128(u128),
        code_i8(i8),
        code_i16(i16),
        code_i32(i32),
        code_i64(i64),
        code_i128(i128),
        code_f32(f32),
        code_f64(f64),
        code_char(char),
        code_bool(bool),
        code_unit(()),
        code_str(str),
        code_bytes(bytes),
    );

    /// Begin coding a tuple. This should be followed by coding the
    /// elements with `begin_tuple_elem` followed by a call to `finish_tuple`.
    pub fn begin_tuple(&mut self) -> Result<()> {
        validate_need_matches!(
            self,
            &Schema::Tuple(_) => (),
            "code tuple"
        );
        self.top().api_state =
            ApiState::Tuple {
                next: 0,
            };
        Ok(())
    }

    /// Begin coding an element in a tuple. This should be followed by
    /// coding the inner value. See `begin_tuple`,
    pub fn begin_tuple_elem(&mut self) -> Result<()> {
        let (schema, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Tuple {
                        ref mut next,
                    },
                } => (schema, next),
                "tuple elem"
            );
        let inner_schema =
            match_or_unreachable!(
                schema,
                &Schema::Tuple(ref inners) => inners
            )
            .get(*next)
            .ok_or_else(|| error!(
                "schema non-comformance, begin tuple elem at idx {}, but that is the tuple's len",
                *next,
            ))?;
        *next += 1;
        self.push_need(inner_schema)?;
        Ok(())
    }

    /// Finish coding a tuple. See `begin_tuple`.
    pub fn finish_tuple(&mut self) -> Result<()> {
        let (schema, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Tuple {
                        next,
                    },
                } => (schema, next),
                "tuple finish"
            );
        let inners = 
            match_or_unreachable!(
                schema,
                &Schema::Tuple(ref inners) => inners
            );
        ensure!(
            inners.len() == next,
            "schema non-comformance, finish tuple of len {}, but only encoded {} elems",
            inners.len(),
            next,
        );
        self.pop();
        Ok(())
    }

    /// Begin coding a struct. This should be followed by coding the
    /// fields with `begin_struct_field` followed by a call to `finish_struct`.
    pub fn begin_struct(&mut self) -> Result<()> {
        validate_need_matches!(
            self,
            &Schema::Struct(_) => (),
            "code struct"
        );
        self.top().api_state =
            ApiState::Struct {
                next: 0,
            };
        Ok(())
    }

    /// Begin coding a field in a struct. This should be followed by
    /// coding the inner value. See `begin_struct`,
    pub fn begin_struct_field(&mut self, name: &str) -> Result<()> {
        let (schema, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Struct {
                        ref mut next,
                    },
                } => (schema, next),
                "struct field"
            );
        let field =
            match_or_unreachable!(
                schema,
                &Schema::Struct(ref fields) => fields
            )
            .get(*next)
            .ok_or_else(|| error!(
                "schema non-comformance, begin struct field at idx {}, but that is the struct's len",
                *next,
            ))?;
        ensure!(
            &field.name == name,
            "schema non-comformance, need struct field {:?}, got struct field {:?}",
            field.name,
            name,
        );
        *next += 1;
        self.push_need(&field.inner)?;
        Ok(())
    }

    /// Finish coding a struct. See `begin_struct`.
    pub fn finish_struct(&mut self) -> Result<()> {
        let (schema, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Struct {
                        next,
                    },
                } => (schema, next),
                "struct finish"
            );
        let fields = 
            match_or_unreachable!(
                schema,
                &Schema::Struct(ref fields) => fields
            );
        ensure!(
            fields.len() == next,
            "schema non-comformance, finish struct of len {}, but only coded {} elems",
            fields.len(),
            next,
        );
        self.pop();
        Ok(())
    }

    /// Begin coding an enum. This should be followed by `begin_enum_variant`
    /// followed by coding the inner value, which then auto-finishes the
    /// enum. Returns number of variants.
    pub fn begin_enum(&mut self) -> Result<usize> {
        let num_variants =
            validate_need_matches!(
                self,
                &Schema::Enum(ref variants) => variants.len(),
                "code enum"
            );
        self.top().api_state = ApiState::Enum;
        Ok(num_variants)
    }

    /// Begin coding an enum variant. See `begin_enum`. Returns number of
    /// variants.
    pub fn begin_enum_variant(
        &mut self,
        variant_ord: usize,
        variant_name: &str,
    ) -> Result<usize> {
        let schema =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Enum,
                } => schema,
                "enum variant"
            );
        let variants =
            match_or_unreachable!(
                schema,
                &Schema::Enum(ref variants) => variants
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
        self.top().api_state = ApiState::AutoFinish;
        self.push_need(&variant.inner)?;
        Ok(variants.len())
    }

    /*
    /// Completely code a None value for an Option schema.
    pub fn code_none(&mut self) -> Result<()> {
        validate_need_matches!(
            self,
            &Schema::Option(_) => (),
            "code option"
        );
        self.pop();
        Ok(())
    }

    /// Begin coding a Some value for an Option schema. This should be
    /// followed by coding the inner value, which will auto-finish coding
    /// the Option.
    pub fn begin_some(&mut self) -> Result<()> {
        let inner =
            validate_need_matches!(
                self,
                &Schema::Option(ref inner) => inner,
                "code option"
            );
        self.top().api_state = ApiState::AutoFinish;
        self.push_need(inner)?;
        Ok(())
    }

    /// Begin coding a seq. This should be followed by coding the elements
    /// with `begin_seq_elem` followed by a call to `finish_seq`.
    pub fn begin_seq(&mut self, len: usize) -> Result<()> {
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
            ApiState::Seq {
                declared_len: len,
                encoded_len: 0,
            };
        Ok(self)
    }
    */
}
