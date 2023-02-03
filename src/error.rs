//! Error handling utilities.


macro_rules! error {
    ($($e:tt)*)=>{
        ::std::io::Error::new(
            ::std::io::ErrorKind::Other,
            format!($($e)*),
        )
    };
}

macro_rules! bail {
    ($($e:tt)*)=>{ return Err(error!($($e)*)) };
}

macro_rules! ensure {
    ($c:expr, $($e:tt)*)=>{
        if !$c {
            bail!($($e)*);
        }
    };
}

pub(crate) use error;
pub(crate) use bail;
pub(crate) use ensure;
