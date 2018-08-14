use std::ffi::OsStr;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use nix::sys::signal::{Signal};

use ffi_util::ToCString;
use {Command, Namespace};
use idmap::{UidMap, GidMap};
use stdio::dup_file_cloexec;
use namespace::to_clone_flag;
use caps::Capability;


impl Command {

    /// Allow child process to daemonize. By default we run equivalent of
    /// `set_parent_death_signal(SIGKILL)`. See the `set_parent_death_signal`
    /// for better explanation.
    pub fn allow_daemonize(&mut self) -> &mut Command {
        self.config.death_sig = None;
        self
    }

    /// Set a signal that is sent to a process when it's parent is dead.
    /// This is by default set to `SIGKILL`. And you should keep it that way
    /// unless you know what you are doing.
    ///
    /// Particularly you should consider the following choices:
    ///
    /// 1. Instead of setting ``PDEATHSIG`` to some other signal, send signal
    ///    yourself and wait until child gracefully finishes.
    ///
    /// 2. Instead of daemonizing use ``systemd``/``upstart``/whatever system
    ///    init script to run your service
    ///
    /// Another issue with this option is that it works only with immediate
    /// child. To better control all descendant processes you may need the
    /// following:
    ///
    /// 1. The `prctl(PR_SET_CHILD_SUBREAPER..)` in parent which allows to
    ///    "catch" descendant processes.
    ///
    /// 2. The pid namespaces
    ///
    /// The former is out of scope of this library. The latter works by
    /// ``cmd.unshare(Namespace::Pid)``, but you may need to setup mount points
    /// and other important things (which are out of scope too).
    ///
    /// To reset this behavior use ``allow_daemonize()``.
    ///
    pub fn set_parent_death_signal(&mut self, sig: Signal) -> &mut Command {
        self.config.death_sig = Some(sig);
        self
    }

    /// Set chroot dir. Only absolute path is supported
    ///
    /// This method has a non-standard security feature: even if current_dir
    /// is unspecified we set it to the directory inside the new root dir.
    /// see more details in the description of `Command::current_dir`.
    ///
    /// Note that if both chroot dir and pivot_root specified. The chroot dir
    /// is applied after pivot root. If chroot dir is relative it's relative
    /// to either suffix of the current directory with stripped off pivot dir
    /// or the pivot dir itself (if old workdir is not prefixed by pivot dir)
    ///
    /// # Panics
    ///
    /// If directory is not absolute
    pub fn chroot_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command
    {
        let dir = dir.as_ref();
        if !dir.is_absolute() {
            panic!("Chroot dir must be absolute");
        }
        self.chroot_dir = Some(dir.to_path_buf());

        self
    }

    /// Moves the root of the file system to the directory `put_old` and
    /// makes `new_root` the new root file system. Also it's optionally
    /// unmount `new_root` mount point after moving root (but it must exist
    /// anyway).
    ///
    /// The documentation says that `put_old` must be underneath the
    /// `new_root`.  Currently we have a restriction that both must be absolute
    /// and `new_root` be prefix of `put_old`, but we may lift it later.
    ///
    /// **Warning** if you don't unshare the mount namespace you will get
    /// moved filesystem root for *all processes running in that namespace*
    /// including parent (currently running) process itself. If you don't
    /// run equivalent to ``mount --make-private`` for the old root filesystem
    /// and set ``unmount`` to true, you may get unmounted filesystem for
    /// running processes too.
    ///
    /// See `man 2 pivot` for further details
    ///
    /// Note that if both chroot dir and pivot_root specified. The chroot dir
    /// is applied after pivot root.
    ///
    /// # Panics
    ///
    /// Panics if either path is not absolute or new_root is not a prefix of
    /// put_old.
    pub fn pivot_root<A: AsRef<Path>, B:AsRef<Path>>(&mut self,
        new_root: A, put_old: B, unmount: bool)
        -> &mut Command
    {
        let new_root = new_root.as_ref();
        let put_old = put_old.as_ref();
        if !new_root.is_absolute() {
            panic!("New root must be absolute");
        };
        if !put_old.is_absolute() {
            panic!("The `put_old` dir must be absolute");
        }
        let mut old_cmp = put_old.components();
        for (n, o) in new_root.components().zip(old_cmp.by_ref()) {
            if n != o {
                panic!("The new_root is not a prefix of put old");
            }
        }
        self.pivot_root = Some((new_root.to_path_buf(), put_old.to_path_buf(),
                                unmount));
        self
    }

    /// Unshare given namespaces
    ///
    /// Note: each namespace have some consequences on how new process will
    /// work, some of them are described in the `Namespace` type documentation.
    pub fn unshare<'x>(&mut self, iter: impl IntoIterator<Item=&'x Namespace>)
        -> &mut Command
    {
        for ns in iter {
            self.config.namespaces |= to_clone_flag(*ns);
        }
        self
    }

    /// Reassociate child process with a namespace specified by a file
    /// descriptor
    ///
    /// `file` argument is an open file referring to a namespace
    ///
    /// 'ns' is a namespace type
    ///
    /// See `man 2 setns` for further details
    ///
    /// Note: using `unshare` and `setns` for the same namespace is meaningless.
    pub fn set_namespace<F: AsRawFd>(&mut self, file: &F, ns: Namespace)
        -> io::Result<&mut Command>
    {
        let fd = try!(dup_file_cloexec(file));
        self.config.setns_namespaces.insert(ns, fd);
        Ok(self)
    }

    /// Sets user id and group id mappings for new process
    ///
    /// This automatically enables `User` namespace. You should also set `uid`
    /// and `gid` with respective methods for the new process.
    ///
    /// Note there are basically two ways to enable id maps:
    ///
    /// 1. Write them directly
    /// 2. Invoke a `newuidmap`, `newgidmap` commands
    ///
    /// First option works either if current process is root or if resulting
    /// map only contains current user in the mapping.
    ///
    /// The library will not try to guess the behavior. By default it will
    /// write directly. You need to call the `set_id_map_commands` when you
    /// want non-default behavior.
    ///
    /// See `man 7 user_namespaces` for more info
    pub fn set_id_maps(&mut self, uid_map: Vec<UidMap>, gid_map: Vec<GidMap>)
        -> &mut Command
    {
        self.unshare(&[Namespace::User]);
        self.config.id_maps = Some((uid_map, gid_map));
        self
    }

    /// Set path to command-line utilities for writing uid/gid maps
    ///
    /// The utilities provided my obey same interface as `newuidmap` and
    /// `newgidmap` from `shadow` (or sometimes `uidmap`) package. To get it
    /// working you usually need to setup `/etc/subuid` and `/etc/subgid`
    /// files.
    ///
    /// See `man 1 newuidmap`, `man 1 newgidmap` for details
    ///
    /// This method is no-op unless `set_id_maps` is called.
    pub fn set_id_map_commands<A: AsRef<Path>, B: AsRef<Path>>(&mut self,
        newuidmap: A, newgidmap: B)
        -> &mut Command
    {
        self.id_map_commands = Some((
            newuidmap.as_ref().to_path_buf(),
            newgidmap.as_ref().to_path_buf()));
        self
    }

    /// Keep signal mask intact after executing child, keeps also ignored
    /// signals
    ///
    /// By default signal mask is empty and all signals are reset to the
    /// `SIG_DFL` value right before `execve()` syscall.
    ///
    /// This is only useful if started process is aware of the issue and sets
    /// sigmasks to some reasonable value. When used wisely it may avoid some
    /// race conditions when signal is sent after child is cloned but before
    /// child have been able to establish it's state.
    pub fn keep_sigmask(&mut self) -> &mut Command {
        self.config.restore_sigmask = false;
        self
    }

    /// Set the argument zero for the process
    ///
    /// By default argument zero is same as path to the program to run. You
    /// may set it to a short name of the command or to something else to
    /// pretend there is a symlink to a program (for example to run `gzip` as
    /// `gunzip`).
    pub fn arg0<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.args[0] = arg.to_cstring();
        self
    }

    /// Makes child process a group leader
    ///
    /// If child process is being launched as a foreground job,
    /// the child process group needs to be put into the foreground on
    /// the controlling terminal using `tcsetpgrp`. To request status
    /// information from stopped child process you should call `waitpid` with
    /// `WUNTRACED` flag. And then check status with `WIFSTOPPED` macro.
    /// After giving child process group access to the controlling terminal
    /// you should send the SIGCONT signal to the child process group.
    pub fn make_group_leader(&mut self, make_group_leader: bool) -> &mut Command {
        self.config.make_group_leader = make_group_leader;
        self
    }

    /// Inserts a magic environment variable that will contain pid of spawned
    /// process
    ///
    /// This is usually needed to avoid accidental propagation of the
    /// environment variables targeted only at this specific process.
    ///
    /// # Example
    ///
    /// This is how you can encode [systemd activation] protocol:
    ///
    /// ```rust,ignore
    /// cmd.env_var_with_pid("LISTEN_PID");
    /// cmd.env("LISTEN_FDS", "1");
    /// ```
    ///
    /// [systemd activation]: https://www.freedesktop.org/software/systemd/man/sd_listen_fds.html
    pub fn env_var_with_pid<K>(&mut self, key: K) -> &mut Command
        where K: AsRef<OsStr>,
    {
        self.init_env_map();
        self.environ.as_mut().unwrap().remove(key.as_ref());
        self.pid_env_vars.insert(key.as_ref().to_os_string());
        self
    }

    /// Drop all capabilities, but keep only ones set by this setter
    ///
    /// This method sets three or four sets of capabilities:
    /// * Permitted
    /// * Inherited
    /// * Effective
    /// * Ambient (if supported)
    ///
    /// This works both when uid changes (from 0 to other) and when it
    /// isn't changed, but requires process to have all capabilities
    /// granted by this method.
    ///
    /// This method replaces whole capability mask on each invocation
    pub fn keep_caps<'x>(&mut self,
        caps: impl IntoIterator<Item=&'x Capability>)
    {
        let mut buf = [0u32; 2];
        for item in caps {
            let item = *item as u32;
            buf[(item >> 5) as usize] |= 1 << (item & 31);
        }
        self.keep_caps = Some(buf);
    }
}
