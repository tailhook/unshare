use std::ffi::CString;


pub struct Pivot {
    pub new_root: CString,
    pub put_old: CString,
    pub old_inside: CString,
    pub workdir: CString,
    pub unmount_old_root: bool,
}

pub struct Chroot {
    pub root: CString,
    pub workdir: CString,
}
