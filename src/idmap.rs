use libc::{uid_t, gid_t};


/// Entry (row) in the uid map
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UidMap {
    /// First uid inside the guest namespace
    pub inside_uid: uid_t,
    /// First uid in external (host) namespace
    pub outside_uid: uid_t,
    /// Number of uids that this entry allows starting from inside/outside uid
    pub count: uid_t,
}

/// Entry (row) in the gid map
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GidMap {
    /// First gid inside the guest namespace
    pub inside_gid: gid_t,
    /// First gid in external (host) namespace
    pub outside_gid: gid_t,
    /// Number of gids that this entry allows starting from inside/outside gid
    pub count: gid_t,
}
