use libc::{uid_t, gid_t};


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UidMap {
    pub inside_uid: uid_t,
    pub outside_uid: uid_t,
    pub count: uid_t,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GidMap {
    pub inside_gid: gid_t,
    pub outside_gid: gid_t,
    pub count: gid_t,
}
