use std::ffi::{CString, OsStr};
use std::os::unix::ffi::OsStrExt;


pub trait ToCString {
    fn to_cstring(&self) -> CString;
}

impl<T:AsRef<OsStr>> ToCString for T {
    fn to_cstring(&self) -> CString {
        CString::new(self.as_ref().as_bytes())
        .unwrap()
    }
}

