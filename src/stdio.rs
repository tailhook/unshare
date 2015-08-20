use std::mem;
use std::os::unix::io::{RawFd, FromRawFd, AsRawFd};

use libc;


pub enum Stdio {
    Pipe,
    Inherit,
    Null,
    Fd(Closing),
}

pub struct Closing(RawFd);

impl Stdio {
    pub fn piped() -> Stdio { Stdio::Pipe }
    pub fn inherit() -> Stdio { Stdio::Inherit }
    pub fn null() -> Stdio { Stdio::Null }
}

impl Closing {
    pub fn new(fd: RawFd) -> Closing {
        Closing(fd)
    }
    pub fn into_fd(self) -> RawFd {
        let fd = self.0;
        mem::forget(self);
        return fd;
    }
}

impl FromRawFd for Stdio {
    unsafe fn from_raw_fd(fd: RawFd) -> Stdio {
        return Stdio::Fd(Closing(fd));
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
