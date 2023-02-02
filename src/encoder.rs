
use crate::{
    coder::coder::CoderState,
    var_len::{
        write_var_len_uint,
        write_var_len_sint,
        write_ord,
    },
};
use std::io::{
    Write,
    Result,
};


pub struct Encoder<'a, 'b, W> {
    state: &'b mut CoderState<'a>,
    write: &'b mut W,
}

impl<'a, 'b, W> Encoder<'a, 'b, W> {
    pub fn new(state: &'b mut CoderState<'a>, write: &'b mut W) -> Self {
        Encoder { state, write }
    }
}

macro_rules! encode_le_bytes {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<&mut Self> {
            self.state.$c()?;
            self.write(&n.to_le_bytes())?;
            Ok(self)
        }
    )*};
}

macro_rules! encode_var_len_uint {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<&mut Self> {
            self.state.$c()?;
            write_var_len_uint(&mut self.write, n as u128)?;
            Ok(self)
        }
    )*};
}

macro_rules! encode_var_len_sint {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<&mut Self> {
            self.state.$c()?;
            write_var_len_sint(&mut self.write, n as i128)?;
            Ok(self)
        }
    )*};
}

impl<'a, 'b, W: Write> Encoder<'a, 'b, W> {
    fn write(&mut self, b: &[u8]) -> Result<()> {
        self.write.write_all(b)
    }

    encode_le_bytes!(
        encode_u8(u8) code_u8,
        encode_u16(u16) code_u16,
        encode_i8(i8) code_i8,
        encode_i16(i16) code_i16,
        encode_f32(f32) code_f32,
        encode_f64(f64) code_f64,
    );

    encode_var_len_uint!(
        encode_u32(u32) code_u32,
        encode_u64(u64) code_u64,
        encode_u128(u128) code_u128,
    );

    encode_var_len_sint!(
        encode_i32(i32) code_i32,
        encode_i64(i64) code_i64,
        encode_i128(i128) code_i128,
    );

    pub fn encode_char(&mut self, c: char) -> Result<&mut Self> {
        self.state.code_char()?;
        self.write(&(c as u32).to_le_bytes())?;
        Ok(self)
    }

    pub fn encode_bool(&mut self, b: bool) -> Result<&mut Self> {
        self.state.code_bool()?;
        self.write(&[b as u8])?;
        Ok(self)
    }

    pub fn encode_unit(&mut self) -> Result<&mut Self> {
        self.state.code_unit()?;
        Ok(self)
    }

    pub fn encode_str(&mut self, s: &str) -> Result<&mut Self> {
        self.state.code_str()?;
        write_var_len_uint(&mut self.write, s.len() as u128)?;
        self.write(s.as_bytes())?;
        Ok(self)
    }

    pub fn encode_bytes(&mut self ,s: &[u8]) -> Result<&mut Self> {
        self.state.code_bytes()?;
        write_var_len_uint(&mut self.write, s.len() as u128)?;
        self.write(s)?;
        Ok(self)
    }

    /// Begin encoding a tuple. This should be followed by encoding the
    /// elements with `begin_tuple_elem` followed by a call to `finish_tuple`.
    pub fn begin_tuple(&mut self) -> Result<&mut Self> {
        self.state.begin_tuple()?;
        Ok(self)
    }

    /// Begin encoding an element in a tuple. This should be followed by
    /// encoding the inner value. See `begin_tuple`,
    pub fn begin_tuple_elem(&mut self) -> Result<&mut Self> {
        self.state.begin_tuple_elem()?;
        Ok(self)
    }

    /// Finish encoding a tuple. See `begin_tuple`.
    pub fn finish_tuple(&mut self) -> Result<&mut Self> {
        self.state.finish_tuple()?;
        Ok(self)
    }

    /// Begin encoding a struct. This should be followed by encoding the
    /// fields with `begin_struct_field` followed by a call to `finish_struct`.
    pub fn begin_struct(&mut self) -> Result<&mut Self> {
        self.state.begin_struct()?;
        Ok(self)
    }

    /// Begin encoding a field in a struct. This should be followed by
    /// encoding the inner value. See `begin_struct`,
    pub fn begin_struct_field(&mut self, name: &str) -> Result<&mut Self> {
        self.state.begin_struct_field(name)?;
        Ok(self)
    }

    /// Finish encoding a struct. See `begin_struct`.
    pub fn finish_struct(&mut self) -> Result<&mut Self> {
        self.state.finish_struct()?;
        Ok(self)
    }
    
    /// Begin encoding an enum. This should be followed by `begin_enum_variant`
    /// followed by encoding the inner value, which then auto-finishes the
    /// enum.
    pub fn begin_enum(&mut self) -> Result<&mut Self> {
        self.state.begin_enum()?;
        Ok(self)
    }

    /// Begin coding an enum variant. See `begin_enum`.
    pub fn begin_enum_variant(
        &mut self,
        variant_ord: usize,
        variant_name: &str,
    ) -> Result<&mut Self> {
        let num_variants = self.state
            .begin_enum_variant(
                variant_ord,
                variant_name,
            )?;
        write_ord(&mut self.write, variant_ord, num_variants)?;
        Ok(self)
    }
}