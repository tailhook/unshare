use std::io;
use std::fmt;
use std::error::Error as StdError;
use status::ExitStatus;

use nix;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    CreatePipe = 1,
    Fork = 2,
    Exec = 3,
    Chdir = 4,
    ParentDeathSignal = 5,
    PipeError = 6,
    StdioError = 7,
    SetUser = 8,
    ChangeRoot = 9,
    SetIdMap = 10,
    SetPGid = 11,
    SetNs = 12,
    CapSet = 13,
}

/// Error runnning process
///
/// This type has very large number of options and it's enum only to be
/// compact. Probably you shouldn't match on the error cases but just format
/// it for user into string.
#[derive(Debug)]
pub enum Error {
    /// Unknown nix error
    ///
    /// Frankly, this error should not happen when running process. We just
    /// keep it here in case `nix` returns this error, which should not happen.
    NixError, // Not sure it's possible, but it is here to convert from
                 // nix::Error safer
    /// Some invalid error code received from child application
    UnknownError,
    /// Error happened when we were trying to create pipe. The pipes used for
    /// two purposes: (a) for the process's stdio (`Stdio::pipe()` or
    /// `Stdio::null()`), (b) internally to wake up child process and return
    /// error back to the parent.
    // TODO(tailhook) should create pipe be split into PipeError and StdioError
    CreatePipe(i32),
    /// Error when forking/cloning process
    Fork(i32),
    /// Error when running execve() systemcall
    Exec(i32),
    /// Error when setting working directory specified by user
    Chdir(i32),
    /// Unable to set death signal (probably signal number invalid)
    ParentDeathSignal(i32),
    /// Error reading/writing through one of the two signal pipes
    PipeError(i32),
    /// Error waiting for process (for some functions only, for example
    /// ``Command::status()``). It probably means someone already waited for
    /// the process, for example it might be other thread, or signal handler.
    WaitError(i32),
    /// Error setting up stdio for process
    StdioError(i32),
    /// Could not set supplementary groups, group id  or user id for the
    /// process
    SetUser(i32),
    /// Error changing root, it explains `chroot`, `pivot_root` system calls
    /// and setting working directory inside new root. Also includes unmounting
    /// old file system for pivot_root case.
    ChangeRoot(i32),
    /// Error setting uid or gid map. May be either problem running
    /// `newuidmap`/`newgidmap` command or writing the mapping file directly
    SetIdMap(i32),
    /// Auxillary command failed
    ///
    /// There are two auxillary commands for now: `newuidmap` and `newgidmap`.
    /// They run only when uid mappings (user namespaces) are enabled.
    ///
    /// Note that failing to run the binary results to `SedIdMap(sys_errno)`,
    /// this error contains status code of command that was succesfullly
    /// spawned.
    AuxCommandExited(i32),
    /// Auxillary command was killed by signal
    ///
    /// Similar to `AuxCommandExited` but when command was killed
    AuxCommandKilled(i32),
    /// Error when calling setpgid function
    SetPGid(i32),
    /// Error when calling setns syscall
    SetNs(i32),
    /// Error when calling capset syscall
    CapSet(i32),
    /// Before unfreeze callback error
    BeforeUnfreeze(Box<::std::error::Error + Send + Sync + 'static>),
}

impl Error {
    /// Similarly to `io::Error` returns bare error code
    pub fn raw_os_error(&self) -> Option<i32> {
        use self::Error::*;
        match self {
            &UnknownError => None,
            &NixError => None,
            &CreatePipe(x) => Some(x),
            &Fork(x) => Some(x),
            &Exec(x) => Some(x),
            &Chdir(x) => Some(x),
            &ParentDeathSignal(x) => Some(x),
            &PipeError(x) => Some(x),
            &WaitError(x) => Some(x),
            &StdioError(x) => Some(x),
            &SetUser(x) => Some(x),
            &ChangeRoot(x) => Some(x),
            &SetIdMap(x) => Some(x),
            &AuxCommandExited(..) => None,
            &AuxCommandKilled(..) => None,
            &SetPGid(x) => Some(x),
            &SetNs(x) => Some(x),
            &CapSet(x) => Some(x),
            &BeforeUnfreeze(..) => None,
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &'static str {
        use self::Error::*;
        match self {
            &UnknownError => "unexpected value received via signal pipe",
            &NixError => "some unknown nix error",
            &CreatePipe(_) => "can't create pipe",
            &Fork(_) => "error when forking",
            &Exec(_) => "error when executing",
            &Chdir(_) => "error when setting working directory",
            &ParentDeathSignal(_) => "error when death signal",
            &PipeError(_) => "error in signalling pipe",
            &WaitError(_) => "error in waiting for child",
            &StdioError(_) => "error setting up stdio for child",
            &SetUser(_) => "error setting user or groups",
            &ChangeRoot(_) => "error changing root directory",
            &SetIdMap(_) => "error setting uid/gid mappings",
            &AuxCommandExited(_) => "aux command exited with non-zero code",
            &AuxCommandKilled(_) => "aux command was killed by signal",
            &SetPGid(_) => "error when calling setpgid",
            &SetNs(_) => "error when calling setns",
            &CapSet(_) => "error when setting capabilities",
            &BeforeUnfreeze(_) => "error in before_unfreeze callback",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if let Some(code) = self.raw_os_error() {
            let errno = nix::errno::from_i32(code);
            if let nix::errno::Errno::UnknownErrno = errno {
                // May be OS knows error name better
                write!(fmt, "{}: {}", self.description(),
                    io::Error::from_raw_os_error(code))
            } else {
                // Format similar to that of std::io::Error
                write!(fmt, "{}: {} (os error {})", self.description(),
                    errno.desc(), code)
            }
        } else {
            write!(fmt, "{}", self.description())
        }
    }
}

#[inline]
pub fn result<T, E: IntoError>(code: ErrorCode, r: Result<T, E>)
    -> Result<T, Error>
{
    r.map_err(|e| e.into_error(code))
}

#[inline]
pub fn cmd_result<E: IntoError>(def_code: ErrorCode, r: Result<ExitStatus, E>)
    -> Result<(), Error>
{
    match try!(r.map_err(|e| e.into_error(def_code))) {
        ExitStatus::Exited(0) => Ok(()),
        ExitStatus::Exited(x) => Err(Error::AuxCommandExited(x as i32)),
        ExitStatus::Signaled(x, _) => Err(Error::AuxCommandKilled(x as i32)),
    }
}

pub trait IntoError {
    fn into_error(self, code: ErrorCode) -> Error;
}

impl IntoError for nix::Error {
    fn into_error(self, code: ErrorCode) -> Error {
        match self {
            nix::Error::Sys(x) => code.wrap(x as i32),
            _ => Error::NixError,
        }
    }
}

impl IntoError for io::Error {
    fn into_error(self, code: ErrorCode) -> Error {
        code.wrap(self.raw_os_error().unwrap_or(-1))
    }
}

impl IntoError for Error {
    fn into_error(self, code: ErrorCode) -> Error {
        code.wrap(self.raw_os_error().unwrap_or(-1))
    }
}


impl ErrorCode {
    pub fn wrap(self, errno: i32) -> Error {
        use self::ErrorCode as C;
        use self::Error as E;
        match self {
            C::CreatePipe => E::CreatePipe(errno),
            C::Fork => E::Fork(errno),
            C::Exec => E::Exec(errno),
            C::Chdir => E::Chdir(errno),
            C::ParentDeathSignal => E::ParentDeathSignal(errno),
            C::PipeError => E::PipeError(errno),
            C::StdioError => E::StdioError(errno),
            C::SetUser => E::SetUser(errno),
            C::ChangeRoot => E::ChangeRoot(errno),
            C::SetIdMap => E::SetIdMap(errno),
            C::SetPGid => E::SetPGid(errno),
            C::SetNs => E::SetNs(errno),
            C::CapSet => E::CapSet(errno),
        }
    }
    pub fn from_i32(code: i32, errno: i32) -> Error {
        use self::ErrorCode as C;
        use self::Error as E;
        match code {
            c if c == C::CreatePipe as i32 => E::CreatePipe(errno),
            c if c == C::Fork as i32 => E::Fork(errno),
            c if c == C::Exec as i32 => E::Exec(errno),
            c if c == C::Chdir as i32 => E::Chdir(errno),
            c if c == C::ParentDeathSignal as i32
                                                => E::ParentDeathSignal(errno),
            c if c == C::PipeError as i32 => E::PipeError(errno),
            c if c == C::StdioError as i32 => E::StdioError(errno),
            c if c == C::SetUser as i32 => E::SetUser(errno),
            c if c == C::ChangeRoot as i32 => E::ChangeRoot(errno),
            c if c == C::SetIdMap as i32 => E::SetIdMap(errno),
            c if c == C::SetPGid as i32 => E::SetPGid(errno),
            c if c == C::SetNs as i32 => E::SetNs(errno),
            c if c == C::CapSet as i32 => E::CapSet(errno),
            _ => E::UnknownError,
        }
    }
}
