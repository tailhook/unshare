use std::os::unix::io::RawFd;
use std::mem;
use std::ptr;

use libc;
use nix;
use libc::{c_void, c_ulong, sigset_t, size_t};
use libc::{kill, signal};
use libc::{F_GETFD, F_SETFD, F_DUPFD_CLOEXEC, FD_CLOEXEC, MNT_DETACH};
use libc::{SIG_DFL, SIG_SETMASK};

use run::{ChildInfo, MAX_PID_LEN};
use error::ErrorCode as Err;

// And at this point we've reached a special time in the life of the
// child. The child must now be considered hamstrung and unable to
// do anything other than syscalls really.
//
// ESPECIALLY YOU CAN NOT DO MEMORY (DE)ALLOCATIONS
//
// See better explanation at:
// https://github.com/rust-lang/rust/blob/c1e865c/src/libstd/sys/unix/process.rs#L202
//

// In particular ChildInfo is passed by refernce here to avoid
// deallocating (parts of) it.
pub unsafe fn child_after_clone(child: &ChildInfo) -> ! {
    let mut epipe = child.error_pipe;

    child.cfg.death_sig.as_ref().map(|&sig| {
        if libc::prctl(ffi::PR_SET_PDEATHSIG, sig as c_ulong, 0, 0, 0) != 0 {
            fail(Err::ParentDeathSignal, epipe);
        }
    });

    // Now we must wait until parent set some environment for us. It's mostly
    // for uid_map/gid_map. But also used for attaching debugger and maybe
    // other things
    let mut wbuf = [0u8];
    loop {
        // TODO(tailhook) put some timeout on this pipe?
        let rc = libc::read(child.wakeup_pipe,
                            (&mut wbuf).as_ptr() as *mut c_void, 1);
        if rc == 0 {
            // Parent already dead presumably before we had a chance to
            // set PDEATHSIG, so just send signal ourself in that case
            if let Some(sig) = child.cfg.death_sig {
                kill(libc::getpid(), sig as i32);
                libc::_exit(127);
            } else {
                // In case we wanted to daemonize, just continue
                //
                // TODO(tailhook) not sure it's best thing to do. Maybe parent
                // failed to setup uid/gid map for us. Do we want to check
                // specific options? Or should we just always die?
                break;
            }
        } else if rc < 0 {
            let errno = nix::errno::errno();
            if errno == libc::EINTR as i32 ||
               errno == libc::EAGAIN as i32
            {
                    continue;
            } else {
                fail(Err::PipeError, errno);
            }
        } else {
            // Do we need to check that exactly one byte is received?
            break;
        }
    }

    if !child.uid_maps.is_empty() {
        let fd = libc::openat(libc::AT_FDCWD, b"/proc/self/uid_map\0".as_ptr() as *const i8, libc::O_WRONLY);
        if fd < 0 {
            fail(Err::SetIdMap, epipe);
        }
        if libc::write(fd, child.uid_maps.as_ptr() as *const libc::c_void, child.uid_maps.len()) < 0 {
            fail(Err::SetIdMap, epipe);
        }
        if libc::close(fd) < 0 {
            fail(Err::SetIdMap, epipe);
        }
    }

    if !child.gid_maps.is_empty() {
        // first configure setgroups to "deny"
        let fd = libc::openat(libc::AT_FDCWD, b"/proc/self/setgroups\0".as_ptr() as *const i8, libc::O_WRONLY);
        if fd < 0 {
            fail(Err::SetIdMap, epipe);
        }
        let deny = b"deny";
        if libc::write(fd, deny.as_ptr() as *const libc::c_void, deny.len()) < 0 {
            fail(Err::SetIdMap, epipe);
        }
        if libc::close(fd) < 0 {
            fail(Err::SetIdMap, epipe);
        }

        // then write gid_map
        let fd = libc::openat(libc::AT_FDCWD, b"/proc/self/gid_map\0".as_ptr() as *const i8, libc::O_WRONLY);
        if fd < 0 {
            fail(Err::SetIdMap, epipe);
        }
        if libc::write(fd, child.gid_maps.as_ptr() as *const libc::c_void, child.gid_maps.len()) < 0 {
            fail(Err::SetIdMap, epipe);
        }
        if libc::close(fd) < 0 {
            fail(Err::SetIdMap, epipe);
        }
    }

    // Move error pipe file descriptors in case they clobber stdio
    while epipe < 3 {
        let nerr = libc::fcntl(epipe, F_DUPFD_CLOEXEC, 3);
        if nerr < 0 {
            fail(Err::CreatePipe, epipe);
        }
        epipe = nerr;
    }

    for &(nstype, fd) in child.setns_namespaces {
        if libc::setns(fd, nstype.bits()) != 0 {
            fail(Err::SetNs, epipe);
        }
    }

    if !child.pid_env_vars.is_empty() {
        let mut buf = [0u8; MAX_PID_LEN+1];
        let data = format_pid_fixed(&mut buf, libc::getpid());
        for &(index, offset) in child.pid_env_vars {
            // we know that there are at least MAX_PID_LEN+1 bytes in buffer
            child.environ[index].offset(offset as isize)
                .copy_from(data.as_ptr() as *const libc::c_char, data.len());
        }
    }

    child.pivot.as_ref().map(|piv| {
        if ffi::pivot_root(piv.new_root.as_ptr(), piv.put_old.as_ptr()) != 0 {
            fail(Err::ChangeRoot, epipe);
        }
        if libc::chdir(piv.workdir.as_ptr()) != 0 {
            fail(Err::ChangeRoot, epipe);
        }
        if piv.unmount_old_root {
            if libc::umount2(piv.old_inside.as_ptr(), MNT_DETACH) != 0 {
                fail(Err::ChangeRoot, epipe);
            }
        }
    });

    child.chroot.as_ref().map(|chroot| {
        if libc::chroot(chroot.root.as_ptr()) != 0 {
            fail(Err::ChangeRoot, epipe);
        }
        if libc::chdir(chroot.workdir.as_ptr()) != 0 {
            fail(Err::ChangeRoot, epipe);
        }
    });

    child.keep_caps.as_ref().map(|_| {
        // Don't use securebits because on older systems it doesn't work
        if libc::prctl(libc::PR_SET_KEEPCAPS, 1, 0, 0, 0) != 0 {
            fail(Err::CapSet, epipe);
        }
    });

    child.cfg.gid.as_ref().map(|&gid| {
        if libc::setgid(gid) != 0 {
            fail(Err::SetUser, epipe);
        }
    });

    child.cfg.supplementary_gids.as_ref().map(|groups| {
        if libc::setgroups(groups.len() as size_t, groups.as_ptr()) != 0 {
            fail(Err::SetUser, epipe);
        }
    });

    child.cfg.uid.as_ref().map(|&uid| {
        if libc::setuid(uid) != 0 {
            fail(Err::SetUser, epipe);
        }
    });

    child.keep_caps.as_ref().map(|caps| {
        let header = ffi::CapsHeader {
            version: ffi::CAPS_V3,
            pid: 0,
        };
        let data = ffi::CapsData {
            effective_s0: caps[0],
            permitted_s0: caps[0],
            inheritable_s0: caps[0],
            effective_s1: caps[1],
            permitted_s1: caps[1],
            inheritable_s1: caps[1],
        };
        if libc::syscall(libc::SYS_capset, &header, &data) != 0 {
            fail(Err::CapSet, epipe);
        }
        for idx in 0..caps.len()*32 {
            if caps[(idx >> 5) as usize] & (1 << (idx & 31)) != 0 {
                let rc = libc::prctl(
                    libc::PR_CAP_AMBIENT,
                    libc::PR_CAP_AMBIENT_RAISE,
                    idx, 0, 0);
                if rc != 0 && nix::errno::errno() == libc::ENOTSUP {
                    // no need to iterate if ambient caps are notsupported
                    break;
                }
            }
        }
    });

    child.cfg.work_dir.as_ref().map(|dir| {
        if libc::chdir(dir.as_ptr()) != 0 {
            fail(Err::Chdir, epipe);
        }
    });


    for &(dest_fd, src_fd) in child.fds {
        if src_fd == dest_fd {
            let flags = libc::fcntl(src_fd, F_GETFD);
            if flags < 0 ||
                libc::fcntl(src_fd, F_SETFD, flags & !FD_CLOEXEC) < 0
            {
                fail(Err::StdioError, epipe);
            }
        } else {
            if libc::dup2(src_fd, dest_fd) < 0 {
                fail(Err::StdioError, epipe);
            }
        }
    }

    for &(start, end) in child.close_fds {
        if start < end {
            for fd in start..end {
                if child.fds.iter().find(|&&(cfd, _)| cfd == fd).is_none() {
                    // Close may fail with ebadf, and it's okay
                    libc::close(fd);
                }
            }
        }
    }

    if child.cfg.restore_sigmask {
        let mut sigmask: sigset_t = mem::uninitialized();
        libc::sigemptyset(&mut sigmask);
        libc::pthread_sigmask(SIG_SETMASK, &sigmask, ptr::null_mut());
        for sig in 1..32 {
            signal(sig, SIG_DFL);
        }
    }

    if let Some(callback) = child.pre_exec {
        if let Err(e) = callback() {
            fail_errno(Err::PreExec,
                e.raw_os_error().unwrap_or(10873289),
                epipe);
        }
    }

    libc::execve(child.filename,
                 child.args.as_ptr(),
                 // cancelling mutability, it should be fine
                 child.environ.as_ptr() as *const *const libc::c_char);
    fail(Err::Exec, epipe);
}

unsafe fn fail(code: Err, output: RawFd) -> ! {
    fail_errno(code, nix::errno::errno(), output)
}
unsafe fn fail_errno(code: Err, errno: i32, output: RawFd) -> ! {
    let bytes = [
        code as u8,
        (errno >> 24) as u8,
        (errno >> 16) as u8,
        (errno >>  8) as u8,
        (errno >>  0)  as u8,
        // TODO(tailhook) rustc adds a special sentinel at the end of error
        // code. Do we really need it? Assuming our pipes are always cloexec'd.
        ];
    // Writes less than PIPE_BUF should be atomic. It's also unclear what
    // to do if error happened anyway
    libc::write(output, bytes.as_ptr() as *const c_void, 5);
    libc::_exit(127);
}

fn format_pid_fixed<'a>(buf: &'a mut [u8], pid: libc::pid_t) -> &'a [u8] {
    buf[buf.len()-1] = 0;
    if pid == 0 {
        buf[buf.len()-2] = b'0';
        return &buf[buf.len()-2..]
    } else {
        let mut tmp = pid;
        // can't use stdlib function because that can allocate
        for n in (0..buf.len()-1).rev() {
            buf[n] = (tmp % 10) as u8 + b'0';
            tmp /= 10;
            if tmp == 0 {
                return &buf[n..];
            }
        }
        unreachable!("can't format pid");
    };
}
/// We don't use functions from nix here because they may allocate memory
/// which we can't to this this module.
mod ffi {
    use libc::{c_char, c_int};

    pub const PR_SET_PDEATHSIG: c_int = 1;
    pub const CAPS_V3: u32 = 0x20080522;

    #[repr(C)]
    pub struct CapsHeader {
        pub version: u32,
        pub pid: i32,
    }

    #[repr(C)]
    pub struct CapsData {
        pub effective_s0: u32,
        pub permitted_s0: u32,
        pub inheritable_s0: u32,
        pub effective_s1: u32,
        pub permitted_s1: u32,
        pub inheritable_s1: u32,
    }

    extern {
        pub fn pivot_root(new_root: *const c_char, put_old: *const c_char)
            -> c_int;
    }
}

#[cfg(test)]
mod test {
    use rand::{thread_rng, Rng};
    use run::MAX_PID_LEN;
    use std::ffi::CStr;
    use super::format_pid_fixed;

    fn fmt_normal(val: i32) -> String {
        let mut buf = [0u8; MAX_PID_LEN+1];
        let slice = format_pid_fixed(&mut buf, val);
        return CStr::from_bytes_with_nul(slice).unwrap()
            .to_string_lossy().to_string();
    }
    #[test]
    fn test_format() {
        assert_eq!(fmt_normal(0), "0");
        assert_eq!(fmt_normal(1), "1");
        assert_eq!(fmt_normal(7), "7");
        assert_eq!(fmt_normal(79), "79");
        assert_eq!(fmt_normal(254), "254");
        assert_eq!(fmt_normal(1158), "1158");
        assert_eq!(fmt_normal(77839), "77839");
    }
    #[test]
    fn test_random() {
        for _ in 0..100000 {
            let x = thread_rng().gen();
            if x < 0 { continue; }
            assert_eq!(fmt_normal(x), format!("{}", x));
        }
    }
}
