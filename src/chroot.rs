use std::ffi::CString;


pub struct Pivot {
    new_root: CString,
    old_root_outside: CString,
    old_root_inside: CString,
    detach_old_root: bool,
}
