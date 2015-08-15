//! The `Command` has mostly same API as `std::process::Command` except
//! where is absolutely needed.
//!
//! In addition `Command` contains methods to configure linux namespaces,
//! chroots and more linux stuff.
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
mod child;
mod run;

use std::ffi::{CString, OsString};
use std::collections::HashMap;
use std::process::Stdio;

use libc::{pid_t};

pub struct Command {
    filename: CString,
    args: Vec<CString>,
    environ: Option<HashMap<OsString, OsString>>,
    config: config::Config,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
}

#[derive(Debug)]
pub struct Child {
    pid: pid_t,
    //status: Option<ExitStatus>,
}
