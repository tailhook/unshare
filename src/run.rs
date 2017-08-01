use std::io::{Read, Write};
use std::ptr;
use std::fs::File;
use std::env::current_dir;
use std::path::{Path, PathBuf};
use std::ffi::CString;
use std::os::unix::io::{RawFd, AsRawFd};
use std::os::unix::ffi::{OsStrExt};
use std::collections::HashMap;

use libc::{c_char, close};
use nix;
use nix::errno::EINTR;
use nix::fcntl::{fcntl, FcntlArg};
use nix::fcntl::{open, O_CLOEXEC, O_RDONLY, O_WRONLY};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::{SIGKILL, SIGCHLD, kill};
use nix::sys::stat::Mode;
use nix::sys::wait::waitpid;
use nix::unistd::{setpgid, Pid};

use child;
use config::Config;
use {Command, Child, ExitStatus};
use error::{Error, result, cmd_result};
use error::ErrorCode as Err;
use pipe::{Pipe, PipeReader, PipeWriter, PipeHolder};
use stdio::{Fd, Closing};
use chroot::{Pivot, Chroot};
use ffi_util::ToCString;
use namespace::to_clone_flag;


pub struct ChildInfo<'a> {
    pub filename: *const c_char,
    pub args: &'a [*const c_char],
    pub environ: &'a [*const c_char],
    pub cfg: &'a Config,
    pub chroot: &'a Option<Chroot>,
    pub pivot: &'a Option<Pivot>,
    pub wakeup_pipe: RawFd,
    pub error_pipe: RawFd,
    pub fds: &'a [(RawFd, RawFd)],
    /// This map may only be used for lookup but not for iteration!
    pub fd_lookup: &'a HashMap<RawFd, RawFd>,
    pub close_fds: &'a [(RawFd, RawFd)],
    pub setns_namespaces: &'a [(CloneFlags, RawFd)],
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
    let mut dircmp = dir.components();
    for (dc, rc) in rel.components().zip(dircmp.by_ref()) {
        if dc != rc {
            return None;
        }
    }
    if absolute {
        Some(Path::new("/").join(dircmp.as_path()))
    } else {
        Some(dircmp.as_path().to_path_buf())
    }
}

fn prepare_descriptors(fds: &HashMap<RawFd, Fd>)
    -> Result<(HashMap<RawFd, RawFd>, HashMap<RawFd, PipeHolder>,
               Vec<Closing>), Error>
{
    let mut inner = HashMap::new();
    let mut outer = HashMap::new();
    let mut guards = Vec::new();
    for (&dest_fd, fdkind) in fds.iter() {
        let mut fd = match fdkind {
            &Fd::ReadPipe => {
                let (rd, wr) = try!(Pipe::new()).split();
                let fd = rd.into_fd();
                guards.push(Closing::new(fd));
                outer.insert(dest_fd, PipeHolder::Writer(wr));
                fd
            }
            &Fd::WritePipe => {
                let (rd, wr) = try!(Pipe::new()).split();
                let fd = wr.into_fd();
                guards.push(Closing::new(fd));
                outer.insert(dest_fd, PipeHolder::Reader(rd));
                fd
            }
            &Fd::ReadNull => {
                // Need to keep fd with cloexec, until we are in child
                let fd = try!(result(Err::CreatePipe,
                    open(Path::new("/dev/null"), O_CLOEXEC|O_RDONLY,
                         Mode::empty())));
                guards.push(Closing::new(fd));
                fd
            }
            &Fd::WriteNull => {
                // Need to keep fd with cloexec, until we are in child
                let fd = try!(result(Err::CreatePipe,
                    open(Path::new("/dev/null"), O_CLOEXEC|O_WRONLY,
                         Mode::empty())));
                guards.push(Closing::new(fd));
                fd
            }
            &Fd::Inherit => {
                dest_fd
            }
            &Fd::Fd(ref x) => {
                x.as_raw_fd()
            }
        };
        // The descriptor must not clobber the descriptors that are passed to
        // a child
        while fd != dest_fd && fds.contains_key(&fd) {
            fd = try!(result(Err::CreatePipe,
                fcntl(fd, FcntlArg::F_DUPFD_CLOEXEC(3))));
            guards.push(Closing::new(fd));
        }
        inner.insert(dest_fd, fd);
    }
    Ok((inner, outer, guards))
}

impl Command {
    /// Run the command and return exit status
    pub fn status(&mut self) -> Result<ExitStatus, Error> {
        // TODO(tailhook) stdin/stdout/stderr
        try!(self.spawn())
        .wait()
        .map_err(|e| Error::WaitError(e.raw_os_error().unwrap_or(-1)))
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
        let (wakeup_rd, wakeup) = try!(Pipe::new()).split();
        let (errpipe, errpipe_wr) = try!(Pipe::new()).split();

        let c_args = raw_with_null(&self.args);

        let environ: Vec<CString> = self.environ.as_ref().unwrap()
            .iter().map(|(k, v)| {
                let mut pair = k[..].as_bytes().to_vec();
                pair.push(b'=');
                pair.extend(v.as_bytes());
                CString::new(pair).unwrap()
            }).collect();
        let c_environ: Vec<_> = raw_with_null(&environ);

        let (int_fds, ext_fds, _guards) = try!(prepare_descriptors(&self.fds));

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

        let mut nstack = [0u8; 4096];
        let mut wakeup = Some(wakeup);
        let mut wakeup_rd = Some(wakeup_rd);
        let mut errpipe_wr = Some(errpipe_wr);
        let args_slice = &c_args[..];
        let environ_slice = &c_environ[..];
        // We transform all hashmaps into vectors, because iterating over
        // hash map involves closure which crashes in the child in unoptimized
        // build
        let fds = int_fds.iter().map(|(&x, &y)| (x, y)).collect::<Vec<_>>();
        let close_fds = self.close_fds.iter().cloned().collect::<Vec<_>>();
        let setns_ns = self.config.setns_namespaces.iter()
            .map(|(ns, fd)| (to_clone_flag(*ns), fd.as_raw_fd()))
            .collect::<Vec<_>>();
        let pid = try!(result(Err::Fork, clone(Box::new(|| -> isize {
            // Note: mo memory allocations/deallocations here
            close(wakeup.take().unwrap().into_fd());
            let child_info = ChildInfo {
                filename: self.filename.as_ptr(),
                args: args_slice,
                environ: environ_slice,
                cfg: &self.config,
                chroot: &chroot,
                pivot: &pivot,
                wakeup_pipe: wakeup_rd.take().unwrap().into_fd(),
                error_pipe: errpipe_wr.take().unwrap().into_fd(),
                fds: &fds,
                fd_lookup: &int_fds,
                close_fds: &close_fds,
                setns_namespaces: &setns_ns,
            };
            child::child_after_clone(&child_info);
        }), &mut nstack[..], self.config.namespaces, Some(SIGCHLD as i32))));
        drop(wakeup_rd);
        drop(errpipe_wr); // close pipe so we don't wait for ourself

        if let Err(e) = self.after_start(pid, wakeup.unwrap(), errpipe) {
            kill(pid, SIGKILL).ok();
            loop {
                match waitpid(pid, None) {
                    Err(nix::Error::Sys(EINTR)) => continue,
                    _ => break,
                }
            }
            return Err(e);
        }

        let mut outer_fds = ext_fds;
        Ok(Child {
            pid: pid.into(),
            status: None,
            stdin: outer_fds.remove(&0).map(|x| {
                match x {
                    PipeHolder::Writer(x) => x,
                    _ => unreachable!(),
                }}),
            stdout: outer_fds.remove(&1).map(|x| {
                match x {
                    PipeHolder::Reader(x) => x,
                    _ => unreachable!(),
                }}),
            stderr: outer_fds.remove(&2).map(|x| {
                match x {
                    PipeHolder::Reader(x) => x,
                    _ => unreachable!(),
                }}),
            fds: outer_fds,
        })
    }

    fn after_start(&self, pid: Pid,
        mut wakeup: PipeWriter, mut errpipe: PipeReader)
        -> Result<(), Error>
    {
        if self.config.make_group_leader {
            try!(result(Err::SetPGid, setpgid(pid, pid)));
        }

        if let Some(&(ref uids, ref gids)) = self.config.id_maps.as_ref() {
            if let Some(&(ref ucmd, ref gcmd)) = self.id_map_commands.as_ref()
            {
                let mut cmd = Command::new(ucmd);
                cmd.arg(format!("{}", pid));
                for map in uids {
                    cmd.arg(format!("{}", map.inside_uid));
                    cmd.arg(format!("{}", map.outside_uid));
                    cmd.arg(format!("{}", map.count));
                }
                try!(cmd_result(Err::SetIdMap, cmd.status()));
                let mut cmd = Command::new(gcmd);
                cmd.arg(format!("{}", pid));
                for map in gids {
                    cmd.arg(format!("{}", map.inside_gid));
                    cmd.arg(format!("{}", map.outside_gid));
                    cmd.arg(format!("{}", map.count));
                }
                try!(cmd_result(Err::SetIdMap, cmd.status()));
            } else {
                let mut buf = Vec::new();
                for map in uids {
                    writeln!(&mut buf, "{} {} {}",
                        map.inside_uid, map.outside_uid, map.count).unwrap();
                }
                try!(result(Err::SetIdMap,
                    File::create(format!("/proc/{}/uid_map", pid))
                    .and_then(|mut f| f.write_all(&buf[..]))));
                let mut buf = Vec::new();
                for map in gids {
                    writeln!(&mut buf, "{} {} {}",
                        map.inside_gid, map.outside_gid, map.count).unwrap();
                }
                try!(result(Err::SetIdMap,
                    File::create(format!("/proc/{}/gid_map", pid))
                    .and_then(|mut f| f.write_all(&buf[..]))));
            }
        }

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
        Ok(())
    }
}
