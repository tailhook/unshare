use std::io;
use std::mem;
use std::os::unix::io::{RawFd};

use nix::unistd::pipe2;
use nix::fcntl::O_CLOEXEC;
use libc;
use libc::{c_void, size_t};

use error::{result, Error};
use error::ErrorCode::CreatePipe;


pub struct Pipe(RawFd, RawFd);
pub struct PipeReader(RawFd);
pub struct PipeWriter(RawFd);


impl Pipe {
    pub fn new() -> Result<Pipe, Error> {
        let (rd, wr) = try!(result(CreatePipe, pipe2(O_CLOEXEC)));
        Ok(Pipe(rd, wr))
    }
    pub fn into_reader(self) -> PipeReader {
        let Pipe(rd, wr) = self;
        mem::forget(self);
        unsafe { libc::close(wr) };
        return PipeReader(rd);
    }
    pub fn into_writer(self) -> PipeWriter {
        let Pipe(rd, wr) = self;
        mem::forget(self);
        unsafe { libc::close(rd) };
        return PipeWriter(wr);
    }
}

// These methos are used in child context, so no memory allocations and any
// other complex things are allowed
impl Pipe {
    pub unsafe fn into_reader_fd(self) -> i32 {
        let Pipe(rd, wr) = self;
        mem::forget(self);
        libc::close(wr);
        return rd;
    }
    pub unsafe fn into_writer_fd(self) -> i32 {
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

impl Drop for PipeReader {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}

impl io::Read for PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let ret = unsafe {
            libc::read(self.0,
                       buf.as_mut_ptr() as *mut c_void,
                       buf.len() as size_t)
        };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(ret as usize)
    }
}

impl io::Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let ret = unsafe {
            libc::write(self.0,
                        buf.as_ptr() as *const c_void,
                        buf.len() as size_t)
        };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(ret as usize)
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
