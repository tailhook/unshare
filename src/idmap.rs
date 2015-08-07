use std::ffi::CString;
use libc::{uid_t, gid_t};


pub struct UidMap {
    uid: uid_t,
    lower_uid: uid_t,
    count: uid_t,
}

pub struct GidMap {
    gid: gid_t,
    lower_gid: gid_t,
    count: gid_t,
}

pub enum UidMapSetter {
    Command(CString, Vec<UidMap>),
    WriteWrite(Vec<UidMap>),
}

pub enum GidMapSetter {
    Command(CString, Vec<UidMap>),
    WriteWrite(Vec<UidMap>),
}

