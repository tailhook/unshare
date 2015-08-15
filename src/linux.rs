use nix::sys::signal::{SigNum};


use Command;

impl Command {

    /// Allow child process to daemonize. By default we run equivalent of
    /// `set_parent_death_signal(SIGKILL)`. See the `set_parent_death_signal`
    /// for better explanation.
    pub fn allow_daemonize(&mut self) {
        self.config.death_sig = None;
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
    pub fn set_parent_death_signal(&mut self, sig: SigNum) {
        self.config.death_sig = Some(sig);
    }

}
