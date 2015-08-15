use std::io;
use std::ffi::CString;
use std::os::unix::ffi::{OsStringExt, OsStrExt};

use nix;
use libc;
use nix::unistd::pipe2;
use nix::fcntl::O_CLOEXEC;

use super::child;
use {Command, Child, ChildInfo};

fn nixerr<T>(r: nix::Result<T>) -> io::Result<T> {
    r.map_err(|e| match e {
        nix::Error::Sys(eno) => io::Error::from_raw_os_error(eno as i32),
        nix::Error::InvalidPath => {
            panic!("Invalid path somewhere. Must not happen");
        },
    })
}

impl Command {
    pub fn spawn(&mut self) -> io::Result<Child> {
        self.init_env_map();
        unsafe { self.spawn_inner() }
    }

    unsafe fn spawn_inner(&self) -> io::Result<Child> {
        // TODO(tailhook) add RAII for pipes
        let (wakeup_reader, wakeup_writer) = try!(nixerr(pipe2(O_CLOEXEC)));
        let (error_reader, error_writer) = try!(nixerr(pipe2(O_CLOEXEC)));

        let pid = libc::fork();
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }

        let c_args = self.args.iter().map(|a| a.as_ptr()).collect::<Vec<_>>();

        let environ: Vec<CString> = self.environ.as_ref().unwrap()
            .iter().map(|(k, v)| {
                let mut pair = k[..].as_bytes().to_vec();
                pair.push(b'=');
                pair.extend(v.as_bytes());
                CString::new(pair).unwrap()
            }).collect();
        let c_environ: Vec<_> = environ.iter().map(|x| x.as_ptr()).collect();

        let child_info = ChildInfo {
            filename: self.filename.as_ptr(),
            args: &c_args[..],
            environ: &c_environ[..],
            cfg: &self.config,
            wakeup_pipe: wakeup_reader,
            error_pipe: error_writer,
        };

        if pid == 0 {
            child::child_after_clone(&child_info);
        } else {

        }

        Ok(Child {
            pid: pid,
            //status: None,
        })
    }
}
