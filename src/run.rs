use std::io::{Read, Write};
use std::ptr;
use std::env::current_dir;
use std::path::{Path, PathBuf};
use std::ffi::CString;
use std::os::unix::io::{RawFd, AsRawFd};
use std::os::unix::ffi::{OsStrExt};

use libc;
use libc::c_char;
use nix::errno::errno;
use nix::fcntl::{open, O_CLOEXEC, O_RDONLY, O_WRONLY};
use nix::sys::stat::Mode;

use child;
use config::Config;
use {Command, Child, ExitStatus, Stdio};
use error::{Error, result};
use error::ErrorCode as Err;
use pipe::Pipe;
use stdio::Closing;
use chroot::{Pivot, Chroot};
use ffi_util::ToCString;


pub struct ChildInfo<'a> {
    pub filename: *const c_char,
    pub args: &'a [*const c_char],
    pub environ: &'a [*const c_char],
    pub cfg: &'a Config,
    pub chroot: Option<Chroot>,
    pub pivot: Option<Pivot>,
    pub wakeup_pipe: RawFd,
    pub error_pipe: RawFd,
    pub stdin: RawFd,
    pub stdout: RawFd,
    pub stderr: RawFd,
}

fn raw_with_null(arr: &Vec<CString>) -> Vec<*const c_char> {
    let mut vec = Vec::with_capacity(arr.len() + 1);
    for i in arr {
        vec.push(i.as_ptr());
    }
    vec.push(ptr::null());
    return vec;
}

fn relative_to<A:AsRef<Path>, B:AsRef<Path>>(dir: A, rel: B, absolute: bool)
    -> Option<PathBuf>
{
    let dir = dir.as_ref();
    let rel = rel.as_ref();
    let mut relcmp = rel.components();
    for (dc, rc) in dir.components().zip(relcmp.by_ref()) {
        if dc != rc {
            return None;
        }
    }
    if absolute {
        Some(Path::new("/").join(relcmp.as_path()))
    } else {
        Some(relcmp.as_path().to_path_buf())
    }
}

impl Command {
    /// Run the command and return exit status
    pub fn status(&mut self) -> Result<ExitStatus, Error> {
        // TODO(tailhook) stdin/stdout/stderr
        try!(self.spawn())
        .wait()
        .map_err(|e| Error::WaitError(e.raw_os_error().unwrap_or(0)))
    }
    /// Spawn the command and return a handle that can be waited for
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

        let (outer_in, _guard_in, inner_in) = match &self.stdin {
            &Some(Stdio::Pipe) => {
                let (rd, wr) = try!(Pipe::new()).split();
                let fd = rd.into_fd();
                (Some(wr), Some(Closing::new(fd)), fd)
            }
            &Some(Stdio::Null) => {
                // Need to keep fd with cloexec, until we are in child
                let fd = try!(result(Err::CreatePipe,
                    open(Path::new("/dev/null"), O_CLOEXEC|O_RDONLY,
                         Mode::empty())));
                (None, Some(Closing::new(fd)), fd)
            }
            &None | &Some(Stdio::Inherit) => (None, None, 0),
            &Some(Stdio::Fd(ref x)) => (None, None, x.as_raw_fd()),
        };

        let (outer_out, _guard_out, inner_out) = match &self.stdout {
            &Some(Stdio::Pipe) => {
                let (rd, wr) = try!(Pipe::new()).split();
                let fd = wr.into_fd();
                (Some(rd), Some(Closing::new(fd)), fd)
            }
            &Some(Stdio::Null) => {
                // Need to keep fd with cloexec, until we are in child
                let fd = try!(result(Err::CreatePipe,
                    open(Path::new("/dev/null"), O_CLOEXEC|O_WRONLY,
                         Mode::empty())));
                (None, Some(Closing::new(fd)), fd)
            }
            &None | &Some(Stdio::Inherit) => (None, None, 1),
            &Some(Stdio::Fd(ref x)) => (None, None, x.as_raw_fd()),
        };

        let (outer_err, _guard_err, inner_err) = match &self.stderr {
            &Some(Stdio::Pipe) => {
                let (rd, wr) = try!(Pipe::new()).split();
                let fd = wr.into_fd();
                (Some(rd), Some(Closing::new(fd)), fd)
            }
            &Some(Stdio::Null) => {
                // Need to keep fd with cloexec, until we are in child
                let fd = try!(result(Err::CreatePipe,
                    open(Path::new("/dev/null"), O_CLOEXEC|O_WRONLY,
                         Mode::empty())));
                (None, Some(Closing::new(fd)), fd)
            }
            &None | &Some(Stdio::Inherit) => (None, None, 2),
            &Some(Stdio::Fd(ref x)) => (None, None, x.as_raw_fd()),
        };

        let pivot = self.pivot_root.as_ref().map(|&(ref new, ref old, unmnt)| {
            Pivot {
                new_root: new.to_cstring(),
                put_old: old.to_cstring(),
                old_inside: relative_to(old, new, true).unwrap().to_cstring(),
                workdir: current_dir().ok()
                    .and_then(|cur| relative_to(cur, new, true))
                    .unwrap_or(PathBuf::from("/"))
                    .to_cstring(),
                unmount_old_root: unmnt,
            }
        });

        let chroot = self.chroot_dir.as_ref().map(|dir| {
            let wrk_rel = if let Some((ref piv, _, _)) = self.pivot_root {
                piv.join(relative_to(dir, "/", false).unwrap())
            } else {
                dir.to_path_buf()
            };
            Chroot {
                root: dir.to_cstring(),
                workdir: current_dir().ok()
                    .and_then(|cur| relative_to(cur, wrk_rel, true))
                    .unwrap_or(PathBuf::from("/"))
                    .to_cstring()
,
            }
        });

        let pid = libc::fork();
        if pid < 0 {
            return Err(Error::Fork(errno()));
        } else if pid == 0 {
            let child_info = ChildInfo {
                filename: self.filename.as_ptr(),
                args: &c_args[..],
                environ: &c_environ[..],
                cfg: &self.config,
                chroot: chroot,
                pivot: pivot,
                wakeup_pipe: wakeup.into_reader_fd(),
                error_pipe: errpipe.into_writer_fd(),
                stdin: inner_in,
                stdout: inner_out,
                stderr: inner_err,
            };
            child::child_after_clone(&child_info);
        }
        let mut errpipe = errpipe.into_reader();
        let mut wakeup = wakeup.into_writer();

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
            stdin: outer_in,
            stdout: outer_out,
            stderr: outer_err,
        })
    }
}
