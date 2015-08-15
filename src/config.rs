use std::ffi::CString;

use libc::{c_int, uid_t, gid_t};

use idmap::{UidMapSetter, GidMapSetter};
use chroot::Pivot;


#[derive(Default)]
pub struct Config {
    pub death_sig: Option<c_int>,
    pub work_dir: Option<CString>,
    pub chroot_dir: Option<CString>,
    pub pivot_root: Option<Pivot>,  // TODO(tailhook) related to chroot_dir
    pub uid: Option<uid_t>,
    pub gid: Option<gid_t>,
    pub supplementary_gids: Option<Vec<gid_t>>,
    pub namespaces: Option<c_int>,
    pub uid_map: Option<UidMapSetter>,
    pub gid_map: Option<GidMapSetter>,
    // TODO(tailhook) stdin/stdout/stderr file descriptors
    // TODO(tailhook) sigmasks
    // TODO(tailhook) wakeup/error pipe
    // TODO(tailhook) session leader
}
