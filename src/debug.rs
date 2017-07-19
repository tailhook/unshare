use std::fmt::{self, Display};

use nix::sched::CloneFlags;

use Command;


/// This is a builder for various settings of how command may be printed
///
/// Use `format!("{}", cmd.display(style))` to actually print a command.
#[derive(Clone, Debug)]
pub struct Style {
    cmd_only: bool,
    print_env: bool,
    show_path: bool,
}

/// A temporary value returned from `Command::display` for the sole purpose
/// of being `Display`'ed.
pub struct Printer<'a>(&'a Command, &'a Style);

impl Style {
    /// Create a new style object that matches to how `fmt::Debug` works for
    /// the command
    pub fn debug() -> Style {
        Style {
            cmd_only: false,
            print_env: true,
            show_path: true,
        }
    }
    /// Create a simple clean user-friendly display of the command
    ///
    /// Note: this kind of pretty-printing omit many important parts of command
    /// and may be ambiguous.
    pub fn short() -> Style {
        Style {
            cmd_only: true,
            print_env: false,
            show_path: false,
        }
    }
    /// Toggle printing of environment
    ///
    /// When `false` is passed we only show `environ[12]`, i.e. a number of
    /// environment variables. Default is `true` for `Style::debug`
    /// constructor.
    ///
    /// This method does nothing when using `Style::short` construtor
    pub fn env(mut self, enable: bool) -> Style {
        self.print_env = enable;
        self
    }
    /// Toggle printing of full path to the executable
    ///
    /// By default we don't print full executable path in `Style::short` mode.
    ///
    /// Note: if this flag is disabled (default) we only show a name from
    /// `arg0`, instead of executable path. When flag is
    /// enabled, the `arg0` is shown alongside with executable path in
    /// parethesis if the values differ.
    ///
    /// This method does nothing when using `Style::debug` constructor
    pub fn path(mut self, enable: bool) -> Style {
        self.show_path = enable;
        self
    }
}

impl<'a> fmt::Display for Printer<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let Printer(cmd, opt) = *self;

        if opt.cmd_only {
            if opt.show_path {
                write!(fmt, "{:?}", cmd.filename)?;
                if cmd.args[0] != cmd.filename {
                    write!(fmt, " ({:?})", &cmd.args[0])?;
                }
            } else {
                let path = if cmd.args[0] != cmd.filename {
                    &cmd.args[0]
                } else {
                    &cmd.filename
                };
                let last_slash = path.as_bytes().iter()
                    .rposition(|&x| x == b'/');
                if let Some(off) = last_slash {
                    write!(fmt, "{:?}",
                        &String::from_utf8_lossy(&path.as_bytes()[off+1..]))?;
                } else {
                    write!(fmt, "{:?}", path)?;
                }
            }
            for arg in cmd.args[1..].iter() {
                write!(fmt, " {:?}", arg)?;
            }
        } else {
            write!(fmt, "<Command {:?}", cmd.filename)?;
            if cmd.args[0] != cmd.filename {
                write!(fmt, " ({:?})", &cmd.args[0])?;
            }
            for arg in cmd.args[1..].iter() {
                write!(fmt, " {:?}", arg)?;
            }
            if opt.print_env {
                if let Some(ref env) = cmd.environ {
                    write!(fmt, "; environ: {{")?;
                    for (ref k, ref v) in env.iter() {
                        write!(fmt, "{:?}={:?},", k, v)?;
                    }
                    write!(fmt, "}}")?;
                }
            } else {
                if let Some(ref env) = cmd.environ {
                    write!(fmt, "; environ[{}]", env.len())?;
                }
            }
            if let Some(ref dir) = cmd.chroot_dir {
                write!(fmt, "; chroot={:?}", dir)?;
            }
            if let Some((ref new, ref old, unmount)) = cmd.pivot_root {
                write!(fmt, "; pivot_root=({:?};{:?};{})", new, old, unmount)?;
            }
            if cmd.config.namespaces != CloneFlags::empty() {
                // TODO(tailhook)
            }
            if let Some(ref dir) = cmd.config.work_dir {
                write!(fmt, "; work-dir={:?}", dir)?;
            }
            if let Some((ref uidm, ref gidm)) = cmd.config.id_maps {
                write!(fmt, "; uid_map={:?}", uidm)?;
                write!(fmt, "; gid_map={:?}", gidm)?;
            }
            if let Some(ref uid) = cmd.config.uid {
                write!(fmt, "; uid={}", uid)?;
            }
            if let Some(ref gid) = cmd.config.gid {
                write!(fmt, "; gid={}", gid)?;
            }
            if let Some(ref gids) = cmd.config.supplementary_gids {
                write!(fmt, "; gids={:?}", gids)?;
            }
            // TODO(tailhook) stdio, sigchld, death_sig,
            // sigmask, id-map-commands
            write!(fmt, ">")?
        }
        Ok(())
    }
}

impl Command {
    /// Returns the object that implements Display
    pub fn display<'a>(&'a self, style: &'a Style) -> Printer<'a> {
        Printer(self, style)
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        Printer(self, &Style::debug()).fmt(fmt)
    }
}

#[cfg(test)]
mod test {
    use {Command, Style};

    #[test]
    fn test_debug() {
        let mut cmd = Command::new("/bin/hello");
        cmd.env_clear();
        cmd.env("A", "B");
        assert_eq!(&format!("{:?}", cmd),
            r#"<Command "/bin/hello"; environ: {"A"="B",}>"#);
    }

    #[test]
    fn test_comprehensive() {
        let mut cmd = Command::new("/bin/hello");
        cmd.env_clear();
        cmd.env("A", "B");
        assert_eq!(&format!("{}", cmd.display(&Style::debug())),
            r#"<Command "/bin/hello"; environ: {"A"="B",}>"#);
    }

    #[test]
    fn test_pretty() {
        let mut cmd = Command::new("/bin/hello");
        cmd.env_clear();
        cmd.arg("world!");
        assert_eq!(&format!("{}", cmd.display(&Style::short())),
            r#""hello" "world!""#);
    }

    #[test]
    fn test_no_env() {
        let mut cmd = Command::new("/bin/hello");
        cmd.env_clear();
        cmd.env("A", "B");
        assert_eq!(&format!("{}", cmd.display(&Style::debug().env(false))),
            r#"<Command "/bin/hello"; environ[1]>"#);
    }
}
