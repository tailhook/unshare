use std::mem;
use std::os::unix::io::RawFd;

use nix::unistd::pipe2;
use nix::fcntl::O_CLOEXEC;
use libc;

use error::{result, Error};
use error::ErrorCode::CreatePipe;


pub struct Pipe(RawFd, RawFd);


impl Pipe {
    pub fn new() -> Result<Pipe, Error> {
        let (rd, wr) = try!(result(CreatePipe, pipe2(O_CLOEXEC)));
        Ok(Pipe(rd, wr))
    }
}

// These methos are used in child context, so no memory allocations and any
// other complex things are allowed
impl Pipe {
    pub unsafe fn into_reader(self) -> i32 {
        let Pipe(rd, wr) = self;
        mem::forget(self);
        libc::close(wr);
        return rd;
    }
    pub unsafe fn into_writer(self) -> i32 {
        let Pipe(rd, wr) = self;
        mem::forget(self);
        libc::close(rd);
        return wr;
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        let Pipe(x, y) = *self;
        unsafe {
            libc::close(x);
            libc::close(y);
        }
    }
}
