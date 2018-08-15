use std::error::Error;
use libc::pid_t;


impl Command {
    /// Add a callback to run when child is already forked but not yet run
    ///
    /// When starting a child we sometimes need more setup from the parent,
    /// for example: to configure pid namespaces for the unprivileged
    /// process (child) by privileged process (parent).
    ///
    /// This callback runs in **parent** process after all built-in setup is
    /// done (setting uid namespaces).
    ///
    /// If callback returns error, process is shut down.
    fn before_unfreeze(&mut self,
        f: impl FnMut(u32) -> Result<(), Box<Error + Send + Sync + 'static>)
    {
        self.before_unfreeze = Box::new(f);
    }
}
