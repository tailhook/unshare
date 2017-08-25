Rust Unshare
============

*Status:* 90% feature-complete, works in production in [lithos][1] and powers [vagga][2]

[Github](https://github.com/tailhook/unshare) |
[Documentaion](http://docs.rs/unshare) |
[Crate](https://crates.io/crates/unshare)

Unshare is a low-level library to create linux containers.

It contains the following:

* Process creation interface similar to `std::process::Command`
* Unsharing arbitrary linux namespaces
* Ability to change root (`chroot/pivot_root`), `uid`, `gid`, `gid_map`
* Some signal mask handling (especially for new processes)
* Forwarding file descriptors and other unixy stuff (sessions, terminals)
* Setting few important prctl flags (`PR_SET_PDEATHSIG`)
* Runs both as root user and as unprivileged user

Not implemeneted yet:

* Fine grained capabilities control (currently you may change user or use
  user namespaces)

The following is considered:

* Capture input (should be, because part of ``std::process`` interface)
* Pseudo tty creation for child
* The `unshare` and `setns`

The following is out of scope:

* mounting file systems
* setting up network
* in-container and out of container supervision
* handing child signals

[1]: http://lithos.readthedocs.org
[2]: http://vagga.readthedocs.org


License
=======

Licensed under either of

 * Apache License, Version 2.0, (./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license (./LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
