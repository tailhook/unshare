use nix::sched as consts;


/// Namespace name to unshare
///
/// See `man 7 namespaces` for more information
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Namespace {
    /// Unshare the mount namespace. It basically means that you can now mount
    /// and unmount folders without touching parent mount points.
    ///
    /// But note that you also have to make all your mountpoints non-shareable
    /// or changes will be propagated to parent namespace anyway.
    ///
    /// This is always needed if you want `pivot_root` (but not enforced by
    /// library)
    Mount,
    /// Unshare the UTS namespace. This allows you to change hostname of the
    /// new container.
    Uts,
    /// Unshare the IPC namespace. This creates new namespace for System V IPC
    /// POSIX message queues and similar.
    Ipc,
    /// Unshare user namespace. This allows unprivileged user to be root
    /// user in new namespace and/or change mappings between real (outer)
    /// user namespace and the inner one.
    ///
    /// This one is required if you want to unshare any other namespace without
    /// root privileges (it's not enforced by kernel not the library)
    ///
    /// See `man 7 user_namespaces` for more information.
    User,
    /// Unshare pid namespace. The child process becomes PID 1 (inside
    /// container) with the following rough list of consequences:
    ///
    /// 1. All daemon processes are reparented to the process
    /// 2. All signal dispositions are set to `Ignore`. E.g. process doesn't
    ///    get killed by `SIGINT` (Ctrl+C), unless signal handler is explicitly
    ///    set
    /// 3. If the process is dead, all its children are killed by `SIGKILL`
    ///    (i.e. can't catch the death signal)
    ///
    /// All this means that most of the time the new process having this
    /// namespace must be some kind of process supervisor.
    ///
    /// Also take a note that `/proc` is not automatically changed. So you
    /// should also unshare `Mount` namespace and mount new `/proc` inside the
    /// PID namespace.
    ///
    /// See `man 7 pid_namespaces` for more information
    Pid,
    /// Unshare network namespace
    ///
    /// New namespace is empty and has no conectivity, even localhost network,
    /// unless some setup is done afterwards.
    ///
    /// Note that unix sockets continue to work, but "abstract unix sockets"
    /// are isolated as a result of this option. The availability of unix
    /// sockets might also mean that libc is able to resolve DNS names by using
    /// NSCD. You may isolate unix sockets by using any kind of filesystem
    /// isolation.
    Net,
}

impl Namespace {
    /// Convert namespace to a clone flag passed to syscalls
    // TODO(tailhook) should this method be private?
    pub fn to_clone_flag(&self) -> u32 {
        match *self {
            Namespace::Mount => consts::CLONE_NEWNS,
            Namespace::Uts => consts::CLONE_NEWUTS,
            Namespace::Ipc => consts::CLONE_NEWIPC,
            Namespace::User => consts::CLONE_NEWUSER,
            Namespace::Pid => consts::CLONE_NEWPID,
            Namespace::Net => consts::CLONE_NEWNET,
        }
    }
}
