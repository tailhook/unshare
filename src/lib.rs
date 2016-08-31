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
extern crate libc;
extern crate nix;

mod namespace;
mod idmap;
mod chroot;
mod ffi_util;
mod std_api;
mod config;
mod error;
mod pipe;
mod child;
mod linux;
mod fds;
mod run;
mod status;
mod wait;
mod stdio;
mod debug;
mod zombies;

pub use error::Error;
pub use status::ExitStatus;
pub use stdio::{Stdio, Fd};
pub use pipe::{PipeReader, PipeWriter};
pub use namespace::{Namespace};
pub use idmap::{UidMap, GidMap};
pub use zombies::{reap_zombies, child_events, ChildEvent};
pub use nix::sys::signal::SigNum;

use std::ffi::{CString, OsString};
use std::path::PathBuf;
use std::os::unix::io::RawFd;
use std::collections::HashMap;

use pipe::PipeHolder;

use libc::{pid_t};


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
}

impl Drop for Command {
    fn drop(&mut self) {
        for (_, fd) in &self.config.setns_namespaces {
            unsafe { libc::close(*fd) };
        }
    }
}

/// The reference to the running child
#[derive(Debug)]
pub struct Child {
    pid: pid_t,
    status: Option<ExitStatus>,
    fds: HashMap<RawFd, PipeHolder>,
    pub stdin: Option<PipeWriter>,
    pub stdout: Option<PipeReader>,
    pub stderr: Option<PipeReader>,
}
