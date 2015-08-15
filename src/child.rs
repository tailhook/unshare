use std::os::unix::io::RawFd;

use libc;
use nix;
use libc::c_void;

use run::ChildInfo;

// And at this point we've reached a special time in the life of the
// child. The child must now be considered hamstrung and unable to
// do anything other than syscalls really.
//
// ESPECIALLY YOU CAN NOT DO MEMORY ALLOCATIONS
//
// See better explanation at:
// https://github.com/rust-lang/rust/blob/c1e865c/src/libstd/sys/unix/process.rs#L202
//

pub unsafe fn child_after_clone(child: &ChildInfo) -> ! {
    ffi::execve(child.filename,
                child.args.as_ptr(),
                child.environ[..].as_ptr());
    fail(child.error_pipe);
}

unsafe fn fail(output: RawFd) -> ! {
    let errno = nix::errno::errno();
    let bytes = [
        (errno >> 24) as u8,
        (errno >> 16) as u8,
        (errno >>  8) as u8,
        (errno >>  0)  as u8,
        ];
        // TODO(tailhook) rustc adds a special sentinel at the end of error
        // code. Do we really need it? Assuming our pipes are always cloexec'd.
    libc::write(output, bytes.as_ptr() as *const c_void, 4);
    libc::_exit(1);
}

/// We don't use functions from nix here because they may allocate memory
/// which we can't to this this module.
mod ffi {
    use libc::{c_char, c_int};

    extern {
        pub fn execve(path: *const c_char, argv: *const *const c_char,
                      envp: *const *const c_char) -> c_int;
    }
}
