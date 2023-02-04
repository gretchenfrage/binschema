//! Glue between this library and serde. Makes `&mut Encoder` implement
//! `serde::Serializer`. Some notes on how translations occur:
//!
//! - unit structs and unit variants are encoded simply as unit
//! - newtype structs and newtype variants are encoded simply as the inner
//!   value
//! - tuple structs and tuple variants are encoded simply as tuple
//! - upon encoding a seq, uses the `.need()` function to determine whether the
//!   schema expects a fixed len or var len seq, which implies the associated
//!   warning
//! - a map is encoded as a var len seq of (key, value) tuples
//! - when asked to "skip a struct field", it tries encoding a none value for
//!   that field


use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    },
    schema::{
        Schema,
        SeqSchema,
    },
    Encoder,
};
use std::{
    io::Write,
    fmt::Display,
};
use serde::ser::{
    Serialize,
    Serializer,
    SerializeSeq,
    SerializeTuple,
    SerializeTupleStruct,
    SerializeTupleVariant,
    SerializeMap,
    SerializeStruct,
    SerializeStructVariant,
};

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::other(msg.to_string())
    }
}

impl<'a, 'b, 'c, W: Write> Serializer for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.encode_bool(v)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.encode_i8(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.encode_i16(v)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.encode_i32(v)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.encode_i64(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.encode_u8(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.encode_u16(v)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.encode_u32(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.encode_u64(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.encode_f32(v)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.encode_f64(v)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.encode_char(v)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.encode_str(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.encode_bytes(v)
    }

    fn serialize_none(self) -> Result<()> {
        self.encode_none()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_some()?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.encode_unit()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.encode_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.begin_enum(variant_index as usize, variant)?;
        self.encode_unit()
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_enum(variant_index as usize, variant)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self> {
        let len = len
            .ok_or_else(|| Error::other("serialize_seq with None len"))?;
        let is_fixed_len =
            match self.need()? {
                &Schema::Seq(SeqSchema {
                    len: fixed_len,
                    ..
                }) => fixed_len.is_some(),
                need => return Err(Error::new(
                    ErrorKind::SchemaNonConformance,
                    format!("need {:?}, got seq begin", need),
                )),
            };

        if is_fixed_len {
            self.begin_fixed_len_seq(len)?;
        } else {
            self.begin_var_len_seq(len)?;
        }

        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self> {
        self.begin_tuple()?;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.begin_tuple()?;
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.begin_enum(variant_index as usize, variant)?;
        self.begin_tuple()?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self> {
        let len = len
            .ok_or_else(|| Error::other("serialize_map with None len"))?;
        self.begin_var_len_seq(len)?;
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.begin_struct()?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.begin_enum(variant_index as usize, variant)?;
        self.begin_struct()?;
        Ok(self)
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        self.encode_i128(v)
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.encode_u128(v)
    }

    fn is_human_readable(&self) -> bool { false }
}

impl<'a, 'b, 'c, W: Write> SerializeSeq for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_seq_elem()?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.finish_seq()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeTuple for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_tuple_elem()?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.finish_tuple()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeTupleStruct for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_tuple_elem()?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.finish_tuple()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeTupleVariant for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_tuple_elem()?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.finish_tuple()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeMap for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_seq_elem()?;
        self.begin_tuple()?;
        self.begin_tuple_elem()?;
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_tuple_elem()?;
        value.serialize(&mut **self)?;
        self.finish_tuple()
    }

    fn end(self) -> Result<()> {
        self.finish_seq()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeStruct for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_struct_field(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.finish_struct()
    }

    fn skip_field(&mut self, key: &'static str) -> Result<()> {
        self.begin_struct_field(key)?;
        self.encode_none()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeStructVariant for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_struct_field(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.finish_struct()
    }

    fn skip_field(&mut self, key: &'static str) -> Result<()> {
        self.begin_struct_field(key)?;
        self.encode_none()
    }
}

