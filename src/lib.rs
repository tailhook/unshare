extern crate libc;
extern crate nix;

mod namespace;
mod idmap;
mod chroot;
mod ffi_util;
mod std_api;
mod child;
mod run;

use std::os::unix::io::RawFd;
use std::ffi::{CString, OsString};
use std::collections::HashMap;
use std::process::Stdio;

use libc::{c_int, uid_t, gid_t, c_char, pid_t};

use namespace::Namespace;
use idmap::{UidMapSetter, GidMapSetter};
use chroot::Pivot;

#[derive(Default)]
struct Config {
    death_sig: Option<c_int>,
    work_dir: Option<CString>,
    chroot_dir: Option<CString>,
    pivot_root: Option<Pivot>,  // TODO(tailhook) related to chroot_dir
    uid: Option<uid_t>,
    gid: Option<gid_t>,
    supplementary_gids: Option<Vec<gid_t>>,
    namespaces: Option<c_int>,
    uid_map: Option<UidMapSetter>,
    gid_map: Option<GidMapSetter>,
    // TODO(tailhook) stdin/stdout/stderr file descriptors
    // TODO(tailhook) sigmasks
    // TODO(tailhook) wakeup/error pipe
    // TODO(tailhook) session leader
}

pub struct Command {
    filename: CString,
    args: Vec<CString>,
    environ: Option<HashMap<OsString, OsString>>,
    config: Config,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
}

struct ChildInfo<'a> {
    filename: *const c_char,
    args: &'a [*const c_char],
    environ: &'a [*const c_char],
    cfg: &'a Config,
    wakeup_pipe: RawFd,
    error_pipe: RawFd,
    // TODO(tailhook) stdin, stdout, stderr
}

#[derive(Debug)]
pub struct Child {
    pid: pid_t,
    //status: Option<ExitStatus>,
}
