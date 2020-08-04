use std::io;

use {Command, BoxError};


impl Command {
    /// Set a callback to run when child is already forked but not yet run
    ///
    /// When starting a child we sometimes need more setup from the parent,
    /// for example: to configure pid namespaces for the unprivileged
    /// process (child) by privileged process (parent).
    ///
    /// This callback runs in **parent** process after all built-in setup is
    /// done (setting uid namespaces). It always run before ``pre_exec``
    /// callback in child.
    ///
    /// If callback returns error, process is shut down.
    ///
    /// Each invocation **replaces** callback,
    /// so there is only one of them can be called.
    ///
    pub fn before_unfreeze(
        &mut self,
        f: impl FnMut(u32) -> Result<(), BoxError> + 'static,
    ) -> &mut Self {
        self.before_unfreeze = Some(Box::new(f));
        self
    }

    /// Set a callback to run in the child before calling exec
    ///
    /// The callback is executed right before `execve` system calls.
    /// All other modifications of the environment are already applied
    /// at this moment. It always run after ``before_unfreeze`` in parent.
    ///
    /// **Warning** this callback must not do any memory (de)allocations,
    /// use mutexes, otherwise process may crash or deadlock. Only bare
    /// syscalls are allowed (use `libc` crate).
    ///
    /// The closure is allowed to return an I/O error whose
    /// OS error code will be communicated back to the parent
    /// and returned as an error from when the spawn was requested.
    ///
    /// Note: unlike same method in stdlib,
    /// each invocation of this method **replaces** callback,
    /// so there is only one of them can be called.
    pub unsafe fn pre_exec(
        &mut self,
        f: impl Fn() -> io::Result<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.pre_exec = Some(Box::new(f));
        self
    }
}
