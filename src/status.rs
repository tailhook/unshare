use std::fmt;
use nix::sys::signal::SigNum;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// Process exited normally with some exit code
    Exited(i8),
    /// Process was killed by a signal (bool flag is true when core is dumped)
    Signaled(SigNum, /* dore dumped */bool)
}

impl ExitStatus {
    pub fn success(&self) -> bool {
        self == &ExitStatus::Exited(0)
    }
    pub fn code(&self) -> Option<i32> {
        match self {
            &ExitStatus::Exited(e) => Some(e as i32),
            &ExitStatus::Signaled(_, _) => None,
        }
    }
    pub fn signal(&self) -> Option<i32> {
        match self {
            &ExitStatus::Exited(_) => None,
            &ExitStatus::Signaled(sig, _) => Some(sig),
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
                    signal_name(sig).unwrap_or("unknown"), sig)
            }
            &Signaled(sig, true) => {
                write!(fmt, "killed by signal {}[{}] (core dumped)",
                    signal_name(sig).unwrap_or("unknown"), sig)
            }
        }
    }
}

fn signal_name(sig: i32) -> Option<&'static str> {
    use nix::sys::signal as S;
    match sig {
        S::SIGABRT => Some("SIGABRT"),
        S::SIGALRM => Some("SIGALRM"),
        S::SIGEMT  => Some("SIGEMT"),
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
