use std::fmt::{Debug, Formatter, Result};

use Command;


impl Debug for Command {
    fn fmt(&self, fmt: &mut Formatter) -> Result {
        try!(write!(fmt, "<Command {:?}", self.filename));
        if self.args[0] != self.filename {
            try!(write!(fmt, " ({:?})", &self.args[0]));
        }
        for arg in self.args[1..].iter() {
            try!(write!(fmt, " {:?}", arg));
        }
        if let Some(ref env) = self.environ {
            try!(write!(fmt, "; environ: {{"));
            for (ref k, ref v) in env.iter() {
                try!(write!(fmt, "{:?}={:?},", k, v));
            }
            try!(write!(fmt, "}}"));
        }
        if let Some(ref dir) = self.chroot_dir {
            try!(write!(fmt, "; chroot={:?}", dir));
        }
        if let Some((ref new, ref old, unmount)) = self.pivot_root {
            try!(write!(fmt, "; pivot_root=({:?};{:?};{})",
                new, old, unmount));
        }
        if self.config.namespaces != 0 {
            // TODO(tailhook)
        }
        if let Some(ref dir) = self.config.work_dir {
            try!(write!(fmt, "; work-dir={:?}", dir));
        }
        if let Some((ref uidm, ref gidm)) = self.config.id_maps {
            try!(write!(fmt, "; uid_map={:?}", uidm));
            try!(write!(fmt, "; gid_map={:?}", gidm));
        }
        if let Some(ref uid) = self.config.uid {
            try!(write!(fmt, "; uid={}", uid));
        }
        if let Some(ref gid) = self.config.gid {
            try!(write!(fmt, "; gid={}", gid));
        }
        if let Some(ref gids) = self.config.supplementary_gids {
            try!(write!(fmt, "; gids={:?}", gids));
        }
        // TODO(tailhook) stdio, sigchld, death_sig, id-map-commands
        write!(fmt, ">")
    }
}
