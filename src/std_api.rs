// This file was derived from rust's own libstd/process.rs with the following
// copyright:
//
// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
use std::ffi::OsStr;
use std::default::Default;
use std::collections::HashMap;
use std::env;
use std::path::Path;

use libc::{uid_t, gid_t};
use ffi_util::ToCString;
use {Command, Stdio};


impl Command {
    /// Constructs a new `Command` for launching the program at
    /// path `program`, with the following default configuration:
    ///
    /// * No arguments to the program
    /// * Inherit the current process's environment
    /// * Inherit the current process's working directory
    /// * Inherit stdin/stdout/stderr for `spawn` or `status`, but create pipes for `output`
    ///
    /// Builder methods are provided to change these defaults and
    /// otherwise configure the process.
    pub fn new<S: AsRef<OsStr>>(program: S) -> Command {
        Command {
            filename: program.to_cstring(),
            args: vec![program.to_cstring()],
            environ: None,
            config: Default::default(),
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    /// Add an argument to pass to the program.
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.args.push(arg.to_cstring());
        self
    }

    /// Add multiple arguments to pass to the program.
    pub fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> &mut Command {
        self.args.extend(args.iter().map(ToCString::to_cstring));
        self
    }

    // TODO(tailhook) It's only public for our run module any better way?
    pub fn init_env_map(&mut self) {
        if self.environ.is_none() {
            self.environ = Some(env::vars_os().collect());
        }
    }

    /// Inserts or updates an environment variable mapping.
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
        where K: AsRef<OsStr>, V: AsRef<OsStr>
    {
        self.init_env_map();
        self.environ.as_mut().unwrap().insert(
            key.as_ref().to_os_string(),
            val.as_ref().to_os_string());
        self
    }

    /// Removes an environment variable mapping.
    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Command {
        self.init_env_map();
        self.environ.as_mut().unwrap().remove(key.as_ref());
        self
    }

    /// Clears the entire environment map for the child process.
    pub fn env_clear(&mut self) -> &mut Command {
        self.environ = Some(HashMap::new());
        self
    }

    /// Sets the working directory for the child process.
    ///
    /// Note: in case of `chroot` or `pivot_root` the working directory is
    /// always set to something inside the new root. Algorithm is following:
    ///
    /// 1. If path is set to absolute path, current dir is this path *inside*
    ///    the chroot
    /// 2. Check if chroot dir is prefix of `env::current_dir()`. If it is
    ///    set current directory to the suffix. Otherwise set current directory
    ///    to the new root dir.
    /// 3. If `current_dir` is specified (and relative) set working directory
    ///    to the value (i.e. relative to the dir set in #2)
    ///
    /// The `pivot_root` is treated just the same as `chroot`. I.e. we will
    /// not try to set working directory inside the `old_root`, unless path
    /// inside is set explicitly by this method.
    ///
    /// At the end of the day, the ``cmd.current_dir(env::current_dir())`` is
    /// not no-op if using chroot/pivot_root.
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command
    {
        self.config.work_dir = Some(dir.as_ref().to_cstring());
        self
    }

    /// Configuration for the child process's stdin handle (file descriptor 0).
    pub fn stdin(&mut self, cfg: Stdio) -> &mut Command {
        self.stdin = Some(cfg);
        self
    }

    /// Configuration for the child process's stdout handle (file descriptor 1).
    pub fn stdout(&mut self, cfg: Stdio) -> &mut Command {
        self.stdout = Some(cfg);
        self
    }

    /// Configuration for the child process's stderr handle (file descriptor 2).
    pub fn stderr(&mut self, cfg: Stdio) -> &mut Command {
        self.stderr = Some(cfg);
        self
    }

    /// Set user id of the new process. Note that it works only for root
    /// process or if you also set up user namespace
    pub fn uid(&mut self, id: uid_t) -> &mut Command {
        self.config.uid = Some(id);
        self
    }

    /// Set primary group id of the new process. Note that it works only for
    /// root process or if you also set up user namespace
    pub fn gid(&mut self, id: gid_t) -> &mut Command {
        self.config.gid = Some(id);
        self
    }

    /// Set supplementary group ids. Note that it works only for root process
    /// or if you also set up user namespace
    pub fn groups(&mut self, ids: Vec<gid_t>) -> &mut Command {
        self.config.supplementary_gids = Some(ids);
        self
    }
}

