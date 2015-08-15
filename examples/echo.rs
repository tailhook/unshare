extern crate unshare;


fn main() {
    let mut cmd = unshare::Command::new("/bin/echo");
    cmd.arg("hello");
    cmd.arg("world!");

    let child = cmd.spawn().unwrap();
    println!("CHILD {:?}", child);
}
