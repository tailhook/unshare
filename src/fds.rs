use std::mem::zeroed;
use std::ops::{Range, RangeTo, RangeFrom, RangeFull};
use std::os::unix::io::RawFd;

use nix::errno::errno;
use libc::getrlimit;
use libc::RLIMIT_NOFILE;

use stdio::{Fd};
use Command;


/// This is just a temporary enum to coerce `std::ops::Range*` variants
/// into single value for convenience. Used in `close_fds` method.
pub enum AnyRange {
    RangeFrom(RawFd),
    Range(RawFd, RawFd),
}


impl Command {

    /// Configuration for any other file descriptor (panics for fds < 3) use
    /// stdin/stdout/stderr for them
    ///
    /// Rust creates file descriptors with CLOEXEC flag by default, so no
    /// descriptors are inherited except ones specifically configured here
    /// (and stdio which is inherited by default)
    pub fn file_descriptor(&mut self, target_fd: RawFd, cfg: Fd)
        -> &mut Command
    {
        if target_fd <= 2 {
            panic!("Stdio file descriptors must be configured with respective \
                    methods instead of passing fd {} to `file_descritor()`",
                    target_fd)
        }
        self.fds.insert(target_fd, cfg);
        self
    }

    /// Pass Raw file descriptor to the application
    ///
    /// This method assumes that file descriptor is owned by an application
    /// and application is smart enough to keep it until process is started.
    ///
    /// This is useful to avoid `dup()`ing of file descriptors that need to
    /// be hold by process supervisor.
    pub unsafe fn file_descriptor_raw(&mut self, target_fd: RawFd, src: RawFd)
        -> &mut Command
    {
        self.fds.insert(target_fd, Fd::RawFd(src));
        self
    }

    /// Close a range of file descriptors as soon as process forks
    ///
    /// Subsequent calls to this method add additional range. Use `reset_fds`
    /// to remove all the ranges.
    ///
    /// File descriptors that never closed are:
    ///
    /// * the stdio file descriptors
    /// * descriptors configured using `file_descriptor`/`file_descriptor_raw`
    ///   methods
    /// * internal file descriptors used for parent child notification by
    ///   unshare crate itself (they are guaranteed to have CLOEXEC)
    ///
    /// You should avoid this method if possilble and rely on CLOEXEC to
    /// do the work. But sometimes it's inevitable:
    ///
    /// 1. If you need to ensure closing descriptors for security reasons
    /// 2. If you have some bad library out of your control which doesn't
    ///    set CLOEXEC on owned the file descriptors
    ///
    /// Ranges obey the following rules:
    ///
    /// * Range like `..12` is transformed into `3..12`
    /// * Range with undefined upper bound `3..` is capped at current ulimit
    ///   for file descriptors **at the moment of calling the method**
    /// * The full range `..` is an alias to `3..`
    /// * Multiple overlapping ranges are closed multiple times which is
    ///   both harmless and useless
    ///
    /// # Panics
    ///
    /// Panics when can't get rlimit for range without upper bound. Should
    /// never happen in practice.
    ///
    /// Panics when lower range of fd is < 3 (stdio file descriptors)
    ///
    pub fn close_fds<A: Into<AnyRange>>(&mut self, range: A)
        -> &mut Command
    {
        self.close_fds.push(match range.into() {
            AnyRange::Range(x, y) => {
                assert!(x >= 3);
                (x, y)
            }
            AnyRange::RangeFrom(x) => unsafe {
                assert!(x >= 3);
                let mut rlim = zeroed();
                let rc = getrlimit(RLIMIT_NOFILE, &mut rlim);
                if rc < 0 {
                    panic!("Can't get rlimit: errno {}", errno());
                }
                (x, rlim.rlim_cur as RawFd)
            }
        });
        self
    }

    /// Reset file descriptor including stdio to the initial state
    ///
    /// Initial state is inherit all the stdio and do nothing to other fds.
    pub fn reset_fds(&mut self) -> &mut Command {
        self.fds = vec![
                (0, Fd::inherit()),
                (1, Fd::inherit()),
                (2, Fd::inherit()),
                ].into_iter().collect();
        self.close_fds.clear();
        self
    }
}

impl Into<AnyRange> for Range<RawFd> {
    fn into(self) -> AnyRange {
        return AnyRange::Range(self.start, self.end);
    }
}

impl Into<AnyRange> for RangeTo<RawFd> {
    fn into(self) -> AnyRange {
        return AnyRange::Range(3, self.end);
    }
}

impl Into<AnyRange> for RangeFrom<RawFd> {
    fn into(self) -> AnyRange {
        return AnyRange::RangeFrom(self.start);
    }
}

impl Into<AnyRange> for RangeFull {
    fn into(self) -> AnyRange {
        return AnyRange::RangeFrom(3);
    }
}
