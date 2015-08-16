use std::io::{Read, Write};
use std::ptr;
use std::fs::File;
use std::ffi::CString;
use std::os::unix::io::{RawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt};

use libc;
use libc::c_char;
use nix::errno::errno;

use child;
use config::Config;
use {Command, Child};
use error::{Error, result};
use error::ErrorCode as Err;
use pipe::Pipe;

pub struct ChildInfo<'a> {
    pub filename: *const c_char,
    pub args: &'a [*const c_char],
    pub environ: &'a [*const c_char],
    pub cfg: &'a Config,
    pub wakeup_pipe: RawFd,
    pub error_pipe: RawFd,
    // TODO(tailhook) stdin, stdout, stderr
}

fn raw_with_null(arr: &Vec<CString>) -> Vec<*const c_char> {
    let mut vec = Vec::with_capacity(arr.len() + 1);
    for i in arr {
        vec.push(i.as_ptr());
    }
    vec.push(ptr::null());
    return vec;
}

impl Command {
    pub fn spawn(&mut self) -> Result<Child, Error> {
        // TODO(tailhook) We need mutable self only for init_env_map. Probably
        // we might do this internally and don't modify Command. That would
        // be more clear and also allow to print Display command easily in
        // error handler
        self.init_env_map();
        unsafe { self.spawn_inner() }
    }

    unsafe fn spawn_inner(&self) -> Result<Child, Error> {
        // TODO(tailhook) add RAII for pipes
        let wakeup = try!(Pipe::new());
        let errpipe = try!(Pipe::new());

        let c_args = raw_with_null(&self.args);

        let environ: Vec<CString> = self.environ.as_ref().unwrap()
            .iter().map(|(k, v)| {
                let mut pair = k[..].as_bytes().to_vec();
                pair.push(b'=');
                pair.extend(v.as_bytes());
                CString::new(pair).unwrap()
            }).collect();
        let c_environ: Vec<_> = raw_with_null(&environ);

        let pid = libc::fork();
        if pid < 0 {
            return Err(Error::Fork(errno()));
        } else if pid == 0 {
            let child_info = ChildInfo {
                filename: self.filename.as_ptr(),
                args: &c_args[..],
                environ: &c_environ[..],
                cfg: &self.config,
                wakeup_pipe: wakeup.into_reader(),
                error_pipe: errpipe.into_writer(),
            };
            child::child_after_clone(&child_info);
        }
        let mut errpipe = File::from_raw_fd(errpipe.into_reader());
        let mut wakeup = File::from_raw_fd(wakeup.into_writer());

        try!(result(Err::PipeError, wakeup.write_all(b"x")));
        let mut err = [0u8; 6];
        match try!(result(Err::PipeError, errpipe.read(&mut err))) {
            0 => {}  // Process successfully execve'd or dead
            5 => {
                let code = err[0];
                let errno = ((err[1] as i32) << 24) | ((err[2] as i32) << 16) |
                    ((err[3] as i32) << 8) | (err[4] as i32);
                return Err(Err::from_i32(code as i32, errno))
            }
            _ => { return Err(Error::UnknownError); }
        }

        Ok(Child {
            pid: pid,
            status: None,
        })
    }
}
