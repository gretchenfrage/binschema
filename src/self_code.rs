
use crate::{
    error::Result,
    Encoder,
    Decoder,
};
use std::io::{
    Write,
    Read,
};


/// A type which knows how to encode itself to an encoder. Analogous to
/// `serde::Serialize`.
pub trait SelfEncode {
    fn encode_to<W: Write>(&self, e: &mut Encoder<W>) -> Result<()>;
}

/// A type which knows how to decode itself from a decoder. Analogous to
/// `serde::Deserialize`.
pub trait SelfDecode: Sized {
    fn decode_from<R: Read>(d: &mut Decoder<R>) -> Result<Self>;
}
