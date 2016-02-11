use std::marker::PhantomData;

use libc::pid_t;
use nix::sys::wait::{waitpid, WNOHANG, WUNTRACED, WCONTINUED};
use nix::errno::{EINTR, ECHILD};
use nix::Error;

use {ExitStatus, SigNum};

/// A non-blocking iteration over zombie processes
///
/// Use `reap_zombies()` to create one, and read docs there
pub struct ZombieIterator(PhantomData<u8>);


impl Iterator for ZombieIterator {
    type Item = (pid_t, ExitStatus);

    fn next(&mut self) -> Option<(pid_t, ExitStatus)> {
        use nix::sys::wait::WaitStatus::*;
        loop {
            match waitpid(-1, Some(WNOHANG)) {
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
/// Alternatively see a more comprehensive `child_events()` function.
///
/// # Example
///
/// So waiting for all processes to finish may look like this:
///
/// ```ignore
///     while alive.len() > 0 {
///         sigwait()
///         for (pid, status) in zombies() {
///             alive.remove(pid);
///         }
///     }
/// ```
///
/// # Important Notes
///
/// * If you are using this function you can't reliably use `Child::wait`
///   any more.
/// * If you got `SIGCHLD` you *must* exhaust this iterator until waiting for
///   next signal, or you will have zombie processes around
pub fn reap_zombies() -> ZombieIterator { ZombieIterator(PhantomData) }


/// The event returned from `child_events()` iterator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildEvent {
    /// Child is dead, similar to what returned by `reap_zombies()`
    Death(pid_t, ExitStatus),
    /// Child is stopped on a signal SigNum
    Stop(pid_t, SigNum),
    /// Child is continued (SIGCONT sent)
    Continue(pid_t),
}


/// A non-blocking iteration over zombies and child stops
///
/// Use `child_events()` to create one, and read docs there
pub struct ChildEventsIterator(PhantomData<u8>);


impl Iterator for ChildEventsIterator {
    type Item = ChildEvent;

    fn next(&mut self) -> Option<ChildEvent> {
        use self::ChildEvent::*;
        use nix::sys::wait::WaitStatus::*;
        loop {
            match waitpid(-1, Some(WNOHANG | WUNTRACED | WCONTINUED)) {
                Ok(Exited(pid, status)) => {
                    return Some(Death(pid, ExitStatus::Exited(status)));
                }
                Ok(Signaled(pid, sig, core)) => {
                    return Some(Death(pid, ExitStatus::Signaled(sig, core)));
                }
                Ok(Stopped(pid, sig)) => return Some(Stop(pid, sig)),
                Ok(Continued(pid)) => return Some(Continue(pid)),
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


/// Creates iterator over child events
///
/// On each iteration it calls `waitpid()` and returns one of the
/// events described in `ChildEvent`.
///
/// The operation is non-blocking. The iterator is exhausted when there are no
/// zombie process at the moment.
///
/// Alternatively see a simpler `reap_zombies()` function.
///
/// # Example
///
/// So waiting for all processes to finish may look like this:
///
/// ```ignore
///     while alive.len() > 0 {
///         sigwait()
///         for event in zombies() {
///             match event {
///                 Death(pid, _) => alive.remove(pid),
///                 Stop(..) => {}
///                 Continue(..) => {}
///         }
///     }
/// ```
///
/// # Important Notes
///
/// * If you are using this function you can't reliably use `Child::wait`
///   any more.
/// * If you got `SIGCHLD` you *must* exhaust this iterator until waiting for
///   next signal, or you will have zombie processes around
pub fn child_events() -> ChildEventsIterator {
    ChildEventsIterator(PhantomData)
}
