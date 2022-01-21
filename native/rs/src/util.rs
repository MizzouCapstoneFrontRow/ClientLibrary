macro_rules! unwrap_or_return {
    ( $value:expr, $retval:expr $(,)? ) => {
        // Option<T> -> Option<Option<T>> -> Option<T>
        // Result<T> -> Result<Option<T>> -> Option<T>
        match ($value).map(Some).unwrap_or_default() {
            Some(x) => x,
            None => return $retval,
        }
    };
}

macro_rules! shadow_or_return {
    ( 2 $( $rest:tt )* ) => {
        shadow_or_return!($( $rest )*);
        shadow_or_return!($( $rest )*);
    };
    ( mut $shadow:ident, $retval:expr ) => {
        let mut $shadow = unwrap_or_return!($shadow, $retval);
    };
    ( $shadow:ident, $retval:expr ) => {
        let $shadow = unwrap_or_return!($shadow, $retval);
    };
}

macro_rules! c_str {
    ( $s:literal ) => {
        {
            lazy_static::lazy_static! {
                static ref S: &'static std::ffi::CStr = {
                    let s: &'static str = concat!($s, "\0");
                    let s: &'static [u8] = s.as_bytes();
                    std::ffi::CStr::from_bytes_with_nul(s).unwrap()
                };
            }
            &**S
        }
    }
}
