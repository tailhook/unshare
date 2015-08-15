use std::io;
use std::fmt;
use std::error::Error as StdError;

use nix;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    CreatePipe = 1,
    Fork = 2,
    Exec = 3,
    Chdir = 4,
}

#[derive(Debug, Clone)]
pub enum Error {
    /// Invalid path somewhere when running command. Presumably has embedded
    /// nulls.
    ///
    /// Frankly, this error should not happen when running process. We just
    /// keep it here in case `nix` returns this error, which should not happen.
    InvalidPath, // Not sure it's possible, but it is here to convert from
                 // nix::Error safer
    /// Error happened when we were trying to create pipe. The pipes used for
    /// two purposes: (a) for the process's stdio, (b) internally to wake up
    /// child process and return error back to the parent.
    CreatePipe(i32),
    /// Error when forking process
    Fork(i32),
    /// Error when running execve() systemcall
    Exec(i32),
    /// Error when setting working directory specified by user
    Chdir(i32),
}

impl Error {
    /// Similarly to `io::Error` returns bare error code
    pub fn raw_os_error(&self) -> Option<i32> {
        use self::Error::*;
        match self {
            &InvalidPath => None,
            &CreatePipe(x) => Some(x),
            &Fork(x) => Some(x),
            &Exec(x) => Some(x),
            &Chdir(x) => Some(x),
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
            &Exec(_) => "error when executing",
            &Chdir(_) => "error when setting working directory",
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
            C::Exec => E::Exec(errno),
            C::Chdir => E::Chdir(errno),
        }
    }
}
