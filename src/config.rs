use std::default::Default;
use std::ffi::CString;

use nix::sched::CloneFlags;
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
    pub namespaces: CloneFlags,
    pub restore_sigmask: bool,
    pub make_group_leader: bool,
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
            namespaces: CloneFlags::empty(),
            restore_sigmask: true,
            make_group_leader: false,
        }
    }
}
