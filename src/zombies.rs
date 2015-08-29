use std::marker::PhantomData;

use libc::pid_t;
use nix::sys::wait::{waitpid, WNOHANG};
use nix::errno::{EINTR, ECHILD};
use nix::Error;

use ExitStatus;

/// A non-blocking iteration over zombie processes
///
/// Use `reap_zombies()` to create one, and read docs there
pub struct ZombieIterator(PhantomData<u8>);


impl Iterator for ZombieIterator {
    type Item = (pid_t, ExitStatus);

    fn next(&mut self) -> Option<(pid_t, ExitStatus)> {
        use nix::sys::wait::WaitStatus::*;
        loop {
            match waitpid(0, Some(WNOHANG)) {
                Ok(Exited(pid, status)) => {
                    return Some((pid, ExitStatus::Exited(status)));
                }
                Ok(Signaled(pid, sig, core)) => {
                    return Some((pid, ExitStatus::Signaled(sig, core)));
                }
                Ok(Stopped(_, _)) => continue,
                Ok(Continued(_)) => continue,
                Ok(StillAlive) => return None,
                Err(Error::Sys(EINTR)) => continue,
                Err(Error::Sys(ECHILD)) => return None,
                Err(Error::InvalidPath) => unreachable!(),
                Err(Error::Sys(x)) => {
                    panic!("Unexpected waitpid error: {:?}", x);
                }
            }
        }
    }
}


/// Creates iterator over zombie processes
///
/// On each iteration it calls `waitpid()` and returns child pid and exit
/// status if there is zombie process. The operation is non-blocking. The
/// iterator is exhausted when there are no zombie process at the moment,
///
/// So waiting for all processes to finish may look like this:
/// ```
///     while alive.len() > 0 {
///         sigwait()
///         for (pid, status) in zombies() {
///             alive. remove(pid);
///         }
///     }
/// ```
///
/// Note if you are using this function you can't reliably use `Child::wait`
/// any more.
pub fn reap_zombies() -> ZombieIterator { ZombieIterator(PhantomData) }
