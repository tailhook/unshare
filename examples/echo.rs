extern crate unshare;

use std::process::exit;


fn main() {
    let mut cmd = unshare::Command::new("/bin/echo");
    cmd.arg("hello");
    cmd.arg("world!");

    match cmd.spawn().unwrap().wait().unwrap() {
        // propagate signal
        unshare::ExitStatus::Exited(x) => exit(x as i32),
        unshare::ExitStatus::Signaled(x, _) => exit((128+x) as i32),
    }
}
