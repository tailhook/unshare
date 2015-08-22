use std::default::Default;
use std::ffi::CString;

use nix::sys::signal::{SigNum, SIGKILL};
use libc::{uid_t, gid_t, c_int};

use idmap::{UidMapSetter, GidMapSetter};


pub struct Config {
    pub death_sig: Option<SigNum>,
    pub work_dir: Option<CString>,
    pub uid: Option<uid_t>,
    pub gid: Option<gid_t>,
    pub supplementary_gids: Option<Vec<gid_t>>,
    pub uid_map: Option<UidMapSetter>,
    pub gid_map: Option<GidMapSetter>,
    pub namespaces: u32,
    pub sigchld: bool,
    // TODO(tailhook) sigmasks
    // TODO(tailhook) session leader
}

impl Default for Config {
    fn default() -> Config {
        Config {
            death_sig: Some(SIGKILL),
            work_dir: None,
            uid: None,
            gid: None,
            supplementary_gids: None,
            uid_map: None,
            gid_map: None,
            namespaces: 0,
            sigchld: false,
        }
    }
}
