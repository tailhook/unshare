use std::io;
use std::fmt;
use std::error::Error as StdError;

use nix;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    CreatePipe = 1,
    Fork = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum Error {
    InvalidPath, // Not sure it's possible, but it is here to convert from
                 // nix::Error safer
    CreatePipe(i32),
    Fork(i32),
}

impl Error {
    /// Similarly to io::Error returns bare error code
    pub fn raw_os_error(&self) -> Option<i32> {
        use self::Error::*;
        match self {
            &InvalidPath => None,
            &CreatePipe(x) => Some(x),
            &Fork(x) => Some(x),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &'static str {
        use self::Error::*;
        match self {
            &InvalidPath => "invalid path passed as argument",
            &CreatePipe(_) => "can't create pipe",
            &Fork(_) => "error when forking",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if let Some(code) = self.raw_os_error() {
            let errno = nix::errno::from_i32(code);
            if let nix::errno::Errno::UnknownErrno = errno {
                // May be OS knows error name better
                write!(fmt, "{}: {}", self.description(),
                    io::Error::from_raw_os_error(code))
            } else {
                // Format similar to that of std::io::Error
                write!(fmt, "{}: {} (os error {})", self.description(),
                    errno.desc(), code)
            }
        } else {
            write!(fmt, "{}", self.description())
        }
    }
}

#[inline]
pub fn result<T, E: IntoError>(code: ErrorCode, r: Result<T, E>)
    -> Result<T, Error>
{
    r.map_err(|e| e.into_error(code))
}

pub trait IntoError {
    fn into_error(self, code: ErrorCode) -> Error;
}

impl IntoError for nix::Error {
    fn into_error(self, code: ErrorCode) -> Error {
        match self {
            nix::Error::Sys(x) => code.wrap(x as i32),
            nix::Error::InvalidPath => Error::InvalidPath,
        }
    }
}


impl ErrorCode {
    fn wrap(self, errno: i32) -> Error {
        use self::ErrorCode as C;
        use self::Error as E;
        match self {
            C::CreatePipe => E::CreatePipe(errno),
            C::Fork => E::Fork(errno),
        }
    }
}
