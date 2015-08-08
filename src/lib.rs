extern crate libc;

mod namespace;
mod idmap;
mod chroot;
mod ffi_util;
mod std_api;

use std::ffi::{CString, OsString};
use std::collections::HashMap;
use std::process::Stdio;

use libc::{c_int, uid_t, gid_t};

use namespace::Namespace;
use idmap::{UidMapSetter, GidMapSetter};
use chroot::Pivot;

struct Exec {
    filename: CString,
    args: Vec<CString>,
    environ: Vec<CString>,
}

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
    cfg: Config,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
}
