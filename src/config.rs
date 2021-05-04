use std::default::Default;
use std::ffi::CString;
use std::collections::HashMap;

use nix::sys::signal::{Signal, SIGKILL};
use nix::sched::CloneFlags;
use libc::{uid_t, gid_t};

use crate::idmap::{UidMap, GidMap};
use crate::namespace::Namespace;
use crate::stdio::Closing;


pub struct Config {
    pub death_sig: Option<Signal>,
    pub work_dir: Option<CString>,
    pub uid: Option<uid_t>,
    pub gid: Option<gid_t>,
    pub supplementary_gids: Option<Vec<gid_t>>,
    pub id_maps: Option<(Vec<UidMap>, Vec<GidMap>)>,
    pub namespaces: CloneFlags,
    pub setns_namespaces: HashMap<Namespace, Closing>,
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
            setns_namespaces: HashMap::new(),
            restore_sigmask: true,
            make_group_leader: false,
        }
    }
}
