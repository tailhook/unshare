//! The `Command` has mostly same API as `std::process::Command` except where
//! is absolutely needed.
//!
//! In addition `Command` contains methods to configure linux namespaces,
//! chroots and more linux stuff.
//!
//! We have diverged from ``std::process::Command`` in the following
//! major things:
//!
//! 1. Error handling. Since sometimes we have long chains of system calls
//!    involved, we need to give user some way to find out which call failed
//!    with an error, so `io::Error` is not an option.  We have
//!    ``error::Error`` class which describes the error as precisely as
//!    possible
//!
//! 2. We set ``PDEATHSIG`` to ``SIGKILL`` by default. I.e. child process will
//!    die when parent is dead. This is what you want most of the time. If you
//!    want to allow child process to daemonize explicitly call the
//!    ``allow_daemonize`` method (but look at documentation of
//!    ``Command::set_parent_death_signal`` first).
//!
//! 3. We don't search for `program` in `PATH`. It's hard to do right in all
//!    cases of `chroot`, `pivot_root`, user and mount namespaces. So we expect
//!    its easier to do for your specific container setup.
//!
//! Anyway this is low-level interface. You may want to use some higher level
//! abstraction which mounts filesystems, sets network and monitors processes.
//!
#![warn(missing_docs)]
extern crate libc;
extern crate nix;
#[cfg(test)] extern crate rand;

mod caps;
mod namespace;
mod idmap;
mod chroot;
mod ffi_util;
mod std_api;
mod config;
mod error;
mod pipe;
mod child;
mod callbacks;
mod linux;
mod fds;
mod run;
mod status;
mod wait;
mod stdio;
mod debug;
mod zombies;

pub use crate::error::Error;
pub use crate::status::ExitStatus;
pub use crate::stdio::{Stdio, Fd};
pub use crate::pipe::{PipeReader, PipeWriter};
pub use crate::namespace::{Namespace};
pub use crate::idmap::{UidMap, GidMap};
pub use crate::zombies::{reap_zombies, child_events, ChildEvent};
pub use nix::sys::signal::Signal;
pub use crate::debug::{Style, Printer};
pub use crate::caps::{Capability};

use std::ffi::{CString, OsString};
use std::path::PathBuf;
use std::os::unix::io::RawFd;
use std::collections::{HashMap, HashSet};
use std::io;

use crate::pipe::PipeHolder;

use libc::{pid_t};

type BoxError = Box<dyn (::std::error::Error) + Send + Sync + 'static>;

/// Main class for running processes. Works in the spirit of builder pattern.
pub struct Command {
    filename: CString,
    args: Vec<CString>,
    environ: Option<HashMap<OsString, OsString>>,
    config: config::Config,
    fds: HashMap<RawFd, Fd>,
    close_fds: Vec<(RawFd, RawFd)>,
    chroot_dir: Option<PathBuf>,
    pivot_root: Option<(PathBuf, PathBuf, bool)>,
    id_map_commands: Option<(PathBuf, PathBuf)>,
    pid_env_vars: HashSet<OsString>,
    keep_caps: Option<[u32; 2]>,
    before_unfreeze: Option<Box<dyn FnMut(u32) -> Result<(), BoxError>>>,
    pre_exec: Option<Box<dyn Fn() -> Result<(), io::Error>>>,
}

/// The reference to the running child
#[derive(Debug)]
pub struct Child {
    pid: pid_t,
    status: Option<ExitStatus>,
    fds: HashMap<RawFd, PipeHolder>,
    /// Stdin of a child if it is a pipe
    pub stdin: Option<PipeWriter>,
    /// Stdout of a child if it is a pipe
    pub stdout: Option<PipeReader>,
    /// Stderr of a child if it is a pipe
    pub stderr: Option<PipeReader>,
}
