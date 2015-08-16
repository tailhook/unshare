extern crate unshare;
extern crate argparse;

use std::io::{stderr, Write};
use std::process::exit;

use argparse::{ArgumentParser, Store, StoreOption, Collect, StoreTrue};


fn main() {
    let mut command = "".to_string();
    let mut args: Vec<String> = Vec::new();
    let mut workdir = None::<String>;
    let mut verbose = false;
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
        ap.refer(&mut verbose)
            .add_option(&["-v", "--verbose"], StoreTrue, "
                Enable verbose mode (prints command, pid, exit status)");
        ap.stop_on_first_argument(true);
        ap.parse_args_or_exit();
    }

    let mut cmd = unshare::Command::new(&command);
    cmd.args(&args[..]);
    workdir.map(|dir| cmd.current_dir(dir));
    if verbose {
        // TODO(tailhook) implement display/debug in Command itself
        writeln!(&mut stderr(), "Command {} {:?}", command, args).ok();
    }
    let mut child = match cmd.spawn() {
        Ok(child) => { child }
        Err(e) => {
            writeln!(&mut stderr(), "Error: {}", e).ok();
            exit(127);
        }
    };
    if verbose {
        writeln!(&mut stderr(), "Child pid {}", child.id()).ok();
    }
    let res = child.wait().unwrap();
    if verbose {
        writeln!(&mut stderr(), "[pid {}] {}", child.id(), res).ok();
    }

}
