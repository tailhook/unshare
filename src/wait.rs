use std::io;
use nix::Error;
use nix::sys::wait::waitpid;
use nix::errno::EINTR;

use {Child, ExitStatus};


impl Child {

    /// Returns pid of the process.
    ///
    /// TODO: Should it return pid_t instead of u32? In nightly it returns u32.
    pub fn id(&self) -> u32 {
        self.pid as u32
    }

    /// Synchronously wait for child to complete and return exit status
    pub fn wait(&mut self) -> Result<ExitStatus, io::Error> {
        if let Some(x) = self.status {
            return Ok(x);
        }
        let status = try!(self._wait());
        self.status = Some(status);
        Ok(status)
    }


    pub fn _wait(&mut self) -> Result<ExitStatus, io::Error> {
        use nix::sys::wait::WaitStatus::*;
        loop {
            match waitpid(self.pid, None) {
                Ok(Exited(x, status)) => {
                    assert!(x == self.pid);
                    return Ok(ExitStatus::Exited(status));
                }
                Ok(Signaled(x, sig, core)) => {
                    assert!(x == self.pid);
                    return Ok(ExitStatus::Signaled(sig, core));
                }
                Ok(Stopped(_, _)) => unreachable!(),
                Ok(Continued(_)) => unreachable!(),
                Ok(StillAlive) => unreachable!(),
                Err(Error::Sys(EINTR)) => continue,
                Err(Error::InvalidPath) => unreachable!(),
                Err(Error::Sys(x)) => {
                    return Err(io::Error::from_raw_os_error(x as i32))
                }
            }
        }
    }
}
