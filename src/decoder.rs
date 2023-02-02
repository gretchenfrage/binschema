
use crate::{
    error::error,
    coder::coder::CoderState,
    var_len::{
        read_var_len_uint,
        read_var_len_sint,
    },
};
use std::{
    mem::{
        size_of,
        take,
    },
    io::{
        Read,
        Result,
    },
    borrow::BorrowMut,
    iter::repeat,
};


pub struct Decoder<'a, 'b, R> {
    state: &'b mut CoderState<'a>,
    read: &'b mut R,
}


impl<'a, 'b, R> Decoder<'a, 'b, R> {
    pub fn new(state: &'b mut CoderState<'a>, read: &'b mut R) -> Self {
        Decoder { state, read }
    }
}

macro_rules! decode_le_bytes {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self) -> Result<$t> {
            self.state.$c()?;
            Ok($t::from_le_bytes(self.read([0; size_of::<$t>()])?))
        }
    )*};
}

macro_rules! decode_var_len_uint {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self) -> Result<$t> {
            self.state.$c()?;
            let n = read_var_len_uint(&mut self.read)?;
            $t::try_from(n)
                .map_err(|_| error!(
                    concat!(
                        "malformed data, {} out of range for a ",
                        stringify!($t),
                    ),
                    n,
                ))
        }
    )*};
}

macro_rules! decode_var_len_sint {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self) -> Result<$t> {
            self.state.$c()?;
            let n = read_var_len_sint(&mut self.read)?;
            $t::try_from(n)
                .map_err(|_| error!(
                    concat!(
                        "malformed data, {} out of range for a ",
                        stringify!($t),
                    ),
                    n,
                ))
        }
    )*};
}

impl<'a, 'b, R: Read> Decoder<'a, 'b, R> {
    fn read<B: BorrowMut<[u8]>>(&mut self, mut buf: B) -> Result<B> {
        self.read.read_exact(buf.borrow_mut())?;
        Ok(buf)
    }

    /// Read a varlen-encoded usize.
    fn read_len(&mut self) -> Result<usize> {
        let n = read_var_len_uint(&mut self.read)?;
        usize::try_from(n)
            .map_err(|_| error!(
                "platform limits or malformed data, {} out of range for a usize",
                n,
            ))
    }

    decode_le_bytes!(
        decode_u8(u8) code_u8,
        decode_u16(u16) code_u16,
        decode_i8(i8) code_i8,
        decode_i16(i16) code_i16,
        decode_f32(f32) code_f32,
        decode_f64(f64) code_f64,
    );

    decode_var_len_uint!(
        decode_u32(u32) code_u32,
        decode_u64(u64) code_u64,
        decode_u128(u128) code_u128,
    );

    decode_var_len_sint!(
        decode_i32(i32) code_i32,
        decode_i64(i64) code_i64,
        decode_i128(i128) code_i128,
    );

    pub fn decode_char(&mut self) -> Result<char> {
        self.state.code_char()?;
        let n = u32::from_le_bytes(self.read([0; 4])?);
        char::from_u32(n)
            .ok_or_else(|| error!(
                "malformed data, {} is not a valid char",
                n
            ))
    }

    pub fn decode_bool(&mut self) -> Result<bool> {
        self.state.code_bool()?;
        let [n] = self.read([0])?;
        match n {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(error!("malformed data, {} is not a valid bool", n)),
        }
    }

    pub fn decode_unit(&mut self) -> Result<()> {
        self.state.code_unit()?;
        Ok(())
    }

    /// Clear `buf` and decode a str into it.
    pub fn decode_str_into(&mut self, buf: &mut String) -> Result<()> {
        // always clear the buf, for consistency
        buf.clear();

        self.state.code_str()?;
        let len = self.read_len()?;

        // do a little switcharoo to get ownership of raw Vec<u8> buf
        //
        // this is fine because String::default() won't make any allocs until
        // characters are actually added to it.
        let mut bbuf = take(buf).into_bytes();

        // TODO: protection against malicious payloads

        // try to read all the bytes in
        // on error, make sure to return the buffer
        bbuf.reserve(len);
        bbuf.extend(repeat(0).take(len));
        if let Err(e) = self.read.read_exact(&mut bbuf) {
            bbuf.clear();
            *buf = String::from_utf8(bbuf).unwrap();
            return Err(e);
        }

        // try to convert to utf8
        // on error, make sure to return the buffer
        match String::from_utf8(bbuf) {
            Ok(s) => {
                *buf = s;
                Ok(())
            }
            Err(e) => {
                let mut bbuf = e.into_bytes();
                bbuf.clear();
                *buf = String::from_utf8(bbuf).unwrap();
                Err(error!(
                    "malformed data, non UTF8 str bytes",
                ))
            }
        }
    }

    /// Decode a str into a new alloc.
    pub fn decode_str(&mut self) -> Result<String> {
        let mut buf = String::new();
        self.decode_str_into(&mut buf)?;
        Ok(buf)
    }

    /// Clear `buf` and decode a bytes into it.
    pub fn decode_bytes_into(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        // always clear the buf, for consistency
        buf.clear();

        self.state.code_bytes()?;
        let len = self.read_len()?;
        buf.reserve(len);
        buf.extend(repeat(0).take(len));
        self.read.read_exact(buf)?;
        Ok(())
    }

    /// Decode a bytes into a new alloc.
    pub fn decode_bytes(&mut self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.decode_bytes_into(&mut buf)?;
        Ok(buf)
    }

    /// Begin decoding a tuple. This should be followed by decoding the
    /// elements with `begin_tuple_elem` followed by a call to `finish_tuple`.
    pub fn begin_tuple(&mut self) -> Result<&mut Self> {
        self.state.begin_tuple()?;
        Ok(self)
    }

    /// Begin decoding an element in a tuple. This should be followed by
    /// decoding the inner value. See `begin_tuple`,
    pub fn begin_tuple_elem(&mut self) -> Result<&mut Self> {
        self.state.begin_tuple_elem()?;
        Ok(self)
    }

    /// Finish decoding a tuple. See `begin_tuple`.
    pub fn finish_tuple(&mut self) -> Result<&mut Self> {
        self.state.finish_tuple()?;
        Ok(self)
    }

    /// Begin decoding a struct. This should be followed by decoding the
    /// fields with `begin_struct_field` followed by a call to `finish_struct`.
    pub fn begin_struct(&mut self) -> Result<&mut Self> {
        self.state.begin_struct()?;
        Ok(self)
    }

    /// Begin decoding a field in a struct. This should be followed by
    /// decoding the inner value. See `begin_struct`,
    pub fn begin_struct_field(&mut self, name: &str) -> Result<&mut Self> {
        self.state.begin_struct_field(name)?;
        Ok(self)
    }

    /// Finish decoding a struct. See `begin_struct`.
    pub fn finish_struct(&mut self) -> Result<&mut Self> {
        self.state.finish_struct()?;
        Ok(self)
    }

    /// Begin decoding an enum. Returns the variant This should be followed by `begin_enum_variant`
    /// followed by decoding the inner value, which then auto-finishes the
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
