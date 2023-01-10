
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
macro_rules! ensure {
    ($c:expr, $($e:tt)*)=>{
        if !$c {
            return Err(error!($($e)*));
        }
    };
}

pub use error;
pub use ensure;
