extern crate libc;

mod namespace;
mod idmap;
mod chroot;

use std::ffi::{CString, OsString};
use std::collections::HashMap;

use libc::{c_int, uid_t, gid_t};

use namespace::Namespace;
use idmap::{UidMapSetter, GidMapSetter};
use chroot::Pivot;

struct Exec {
    filename: CString,
    args: Vec<CString>,
    environ: Vec<CString>,
}

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
    // TODO(tailhook) wakeup pipe
    // TODO(tailhook) session leader
}

struct Process {
    exec: Exec,
    environ: Option<HashMap<OsString, OsString>>,
    cfg: Config,
}
