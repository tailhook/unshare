use std::os::unix::io::{RawFd, FromRawFd, AsRawFd};

use libc;


pub enum Stdio {
    Pipe,
    Inherit,
    Null,
    Fd(Closing),
}

pub enum Fd {
    ReadPipe,
    WritePipe,
    Inherit,
    ReadNull,
    WriteNull,
    Fd(Closing),
}

pub struct Closing(RawFd);

impl Stdio {
    pub fn piped() -> Stdio { Stdio::Pipe }
    pub fn inherit() -> Stdio { Stdio::Inherit }
    pub fn null() -> Stdio { Stdio::Null }
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
    pub fn piped_read() -> Fd { Fd::ReadPipe }
    pub fn piped_write() -> Fd { Fd::WritePipe }
    pub fn inherit() -> Fd { Fd::Inherit }
    pub fn read_null() -> Fd { Fd::ReadNull }
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
