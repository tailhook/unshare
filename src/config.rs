use std::default::Default;
use std::ffi::CString;

use nix::sys::signal::{SigNum, SIGKILL};
use libc::{uid_t, gid_t};

use idmap::{UidMap, GidMap};


pub struct Config {
    pub death_sig: Option<SigNum>,
    pub work_dir: Option<CString>,
    pub uid: Option<uid_t>,
    pub gid: Option<gid_t>,
    pub supplementary_gids: Option<Vec<gid_t>>,
    pub id_maps: Option<(Vec<UidMap>, Vec<GidMap>)>,
    pub namespaces: u32,
    pub restore_sigmask: bool,
    pub call_setpgid: bool,
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
            id_maps: None,
            namespaces: 0,
            restore_sigmask: true,
            call_setpgid: false,
        }
    }
}
