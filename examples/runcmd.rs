extern crate unshare;
extern crate argparse;

use std::io::{stderr, Write};

use argparse::{ArgumentParser, Store, StoreOption, Collect};


fn main() {
    let mut command = "".to_string();
    let mut args: Vec<String> = Vec::new();
    let mut workdir = None::<String>;
    {  // this block limits scope of borrows by ap.refer() method
        let mut ap = ArgumentParser::new();
        ap.set_description("Run command with changed process state");
        ap.refer(&mut command)
            .add_argument("command", Store, "Command to run")
            .required();
        ap.refer(&mut args)
            .add_argument("arg", Collect, "Arguments for the command")
            .required();
        ap.refer(&mut workdir)
            .add_option(&["--work-dir"], StoreOption, "
                Set working directory of the command");
        ap.stop_on_first_argument(true);
        ap.parse_args_or_exit();
    }

    let mut cmd = unshare::Command::new(command);
    cmd.args(&args[..]);
    workdir.map(|dir| cmd.current_dir(dir));
    match cmd.spawn() {
        Ok(child) => {
            // TODO(tailhook) wait
            println!("Child spawned {:?}", child);
            ::std::thread::sleep_ms(100);
        }
        Err(e) => {
            write!(&mut stderr(), "Error: {}", e).ok();
        }
    }
}
