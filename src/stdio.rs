use std::os::unix::io::{RawFd, FromRawFd, AsRawFd};

use libc;


/// An enumeration that is used to configure stdio file descritors
///
/// The enumeration members might be non-stable, it's better to use
/// one of the constructors to create an instance
pub enum Stdio {
    Pipe,
    Inherit,
    Null,
    Fd(Closing),
}

/// An enumeration that is used to configure non-stdio file descriptors. It
/// differs from stdio one because we must differentiate from readable and
/// writable file descriptors for things open by the library
///
/// The enumeration members might be non-stable, it's better to use
/// one of the constructors to create an instance
pub enum Fd {
    ReadPipe,
    WritePipe,
    Inherit,
    ReadNull,
    WriteNull,
    Fd(Closing),
    RawFd(RawFd),
}

pub struct Closing(RawFd);

impl Stdio {
    /// Pipe is created for child process
    pub fn piped() -> Stdio { Stdio::Pipe }
    /// The child inherits file descriptor from the parent process
    pub fn inherit() -> Stdio { Stdio::Inherit }
    /// Stream is attached to `/dev/null`
    pub fn null() -> Stdio { Stdio::Null }
    /// Converts stdio definition to file descriptor definition
    /// (mostly needed internally)
    pub fn to_fd(self, write: bool) -> Fd {
        match (self, write) {
            (Stdio::Fd(x), _) => Fd::Fd(x),
            (Stdio::Pipe, false) => Fd::ReadPipe,
            (Stdio::Pipe, true) => Fd::WritePipe,
            (Stdio::Inherit, _) => Fd::Inherit,
            (Stdio::Null, false) => Fd::ReadNull,
            (Stdio::Null, true) => Fd::WriteNull,
        }
    }
}

impl Fd {
    /// Create a pipe so that child can read from it
    pub fn piped_read() -> Fd { Fd::ReadPipe }
    /// Create a pipe so that child can write to it
    pub fn piped_write() -> Fd { Fd::WritePipe }
    /// Inherit the child descriptor from parent
    ///
    /// Not very useful for custom file descriptors better use `from_raw_fd()`
    pub fn inherit() -> Fd { Fd::Inherit }
    /// Create a readable pipe that always has end of file condition
    pub fn read_null() -> Fd { Fd::ReadNull }
    /// Create a writable pipe that ignores all the input
    pub fn write_null() -> Fd { Fd::WriteNull }
}

impl Closing {
    pub fn new(fd: RawFd) -> Closing {
        Closing(fd)
    }
}

impl FromRawFd for Stdio {
    unsafe fn from_raw_fd(fd: RawFd) -> Stdio {
        return Stdio::Fd(Closing(fd));
    }
}

impl FromRawFd for Fd {
    unsafe fn from_raw_fd(fd: RawFd) -> Fd {
        return Fd::Fd(Closing(fd));
    }
}

impl AsRawFd for Closing {
    fn as_raw_fd(&self) -> RawFd {
        return self.0;
    }
}

impl Drop for Closing {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}
