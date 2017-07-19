use std::fmt;
use {Signal};

use nix::c_int;

/// The exit status of a process
///
/// Returned either by `reap_zombies()` or by `child_events()`
/// or by `Child::wait()`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// Process exited normally with some exit code
    Exited(i8),
    /// Process was killed by a signal (bool flag is true when core is dumped)
    Signaled(Signal, /* dore dumped */bool)
}

impl ExitStatus {
    /// Returns `true` if this exit status means successful exit
    pub fn success(&self) -> bool {
        self == &ExitStatus::Exited(0)
    }
    /// Returns exit code if the process has exited normally
    pub fn code(&self) -> Option<i32> {
        match self {
            &ExitStatus::Exited(e) => Some(e as i32),
            &ExitStatus::Signaled(_, _) => None,
        }
    }
    /// Returns signal number if he process was killed by signal
    pub fn signal(&self) -> Option<i32> {
        match self {
            &ExitStatus::Exited(_) => None,
            &ExitStatus::Signaled(sig, _) => Some(sig as i32),
        }
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::ExitStatus::*;
        match self {
            &Exited(c) => write!(fmt, "exited with code {}", c),
            &Signaled(sig, false) => {
                write!(fmt, "killed by signal {}[{}]",
                    signal_name(sig).unwrap_or("unknown"), sig as c_int)
            }
            &Signaled(sig, true) => {
                write!(fmt, "killed by signal {}[{}] (core dumped)",
                    signal_name(sig).unwrap_or("unknown"), sig as c_int)
            }
        }
    }
}

fn signal_name(sig: Signal) -> Option<&'static str> {
    use nix::sys::signal::Signal as S;
    match sig {
        S::SIGABRT => Some("SIGABRT"),
        S::SIGALRM => Some("SIGALRM"),
        S::SIGBUS  => Some("SIGBUS"),
        S::SIGFPE  => Some("SIGFPE"),
        S::SIGHUP  => Some("SIGHUP"),
        S::SIGILL  => Some("SIGILL"),
        S::SIGINT  => Some("SIGINT"),
        S::SIGKILL => Some("SIGKILL"),
        S::SIGPIPE => Some("SIGPIPE"),
        S::SIGQUIT => Some("SIGQUIT"),
        S::SIGSEGV => Some("SIGSEGV"),
        S::SIGTERM => Some("SIGTERM"),
        _ => None,
    }
}
