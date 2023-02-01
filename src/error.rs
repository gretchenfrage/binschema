//! Error handling utilities.


#[macro_export]
macro_rules! error {
    ($($e:tt)*)=>{
        ::std::io::Error::new(
            ::std::io::ErrorKind::Other,
            format!($($e)*),
        )
    };
}

#[macro_export]
macro_rules! bail {
    ($($e:tt)*)=>{ return Err(error!($($e)*)) };
}

#[macro_export]
macro_rules! ensure {
    ($c:expr, $($e:tt)*)=>{
        if !$c {
            bail!($($e)*);
        }
    };
}

pub use error;
pub use bail;
pub use ensure;
