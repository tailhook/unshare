use std::os::unix::io::RawFd;

use libc;
use nix;
use libc::{c_void, c_ulong, size_t};
use libc::funcs::posix88::signal::kill;

use run::ChildInfo;
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
        if ffi::prctl(ffi::PR_SET_PDEATHSIG, sig as c_ulong, 0, 0, 0) != 0 {
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
                kill(libc::getpid(), sig);
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
            if errno == nix::errno::EINTR as i32 ||
               errno == nix::errno::EAGAIN as i32
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

    // Move error pipe file descriptors in case they clobber stdio
    while epipe < 3 {
        let nerr = libc::fcntl(epipe, ffi::F_DUPFD_CLOEXEC);
        if nerr < 0 {
            fail(Err::CreatePipe, epipe);
        }
        epipe = nerr;
    }

    child.cfg.gid.as_ref().map(|&gid| {
        if libc::setgid(gid) != 0 {
            fail(Err::SetUser, epipe);
        }
    });

    child.cfg.supplementary_gids.as_ref().map(|groups| {
        if ffi::setgroups(groups.len() as size_t, groups.as_ptr()) != 0 {
            fail(Err::SetUser, epipe);
        }
    });

    child.cfg.uid.as_ref().map(|&uid| {
        if libc::setuid(uid) != 0 {
            fail(Err::SetUser, epipe);
        }
    });

    child.cfg.work_dir.as_ref().map(|dir| {
        if libc::chdir(dir.as_ptr()) != 0 {
            fail(Err::Chdir, epipe);
        }
    });


    if child.stdin != 0 && libc::dup2(child.stdin, 0) < 0 {
        fail(Err::StdioError, epipe);
    }

    if child.stdout != 1 && libc::dup2(child.stdout, 1) < 0 {
        fail(Err::StdioError, epipe);
    }

    if child.stderr != 2 && libc::dup2(child.stderr, 2) < 0 {
        fail(Err::StdioError, epipe);
    }

    ffi::execve(child.filename,
                child.args.as_ptr(),
                child.environ[..].as_ptr());
    fail(Err::Exec, epipe);
}

unsafe fn fail(code: Err, output: RawFd) -> ! {
    let errno = nix::errno::errno();
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

/// We don't use functions from nix here because they may allocate memory
/// which we can't to this this module.
mod ffi {
    use libc::{c_char, c_int, c_ulong, size_t, gid_t};

    pub const F_DUPFD_CLOEXEC: c_int = 1030;
    pub const PR_SET_PDEATHSIG: c_int = 1;

    extern {
        pub fn execve(path: *const c_char, argv: *const *const c_char,
                      envp: *const *const c_char) -> c_int;
        pub fn prctl(option: c_int, arg2: c_ulong, arg3: c_ulong,
            arg4: c_ulong, arg5: c_ulong) -> c_int;
        pub fn setgroups(size: size_t, gids: *const gid_t) -> c_int;
    }
}
