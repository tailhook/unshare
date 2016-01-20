============
Rust Unshare
============

:Status: 90% feature-complete, works in production in lithos_ and powers vagga_
:Documentation: http://tailhook.github.io/unshare/

Unshare is a low-level library to create linux containers.

It contains the following:

* Process creation interface similar to ``std::process::Command``
* Unsharing arbitrary linux namespaces
* Ability to change root (chroot/pivot_root), uid, gid, gid_map
* Some signal mask handling (especially for new processes)
* Forwarding file descriptors and other unixy stuff (sessions, terminals)
* Setting few important prctl flags (PR_SET_PDEATHSIG)
* Runs both as root user and as unprivileged user

Not implemeneted yet:

* Fine grained capabilities control (currently you may change user or use
  user namespaces)

The following is considered:

* Capture input (should be, because part of ``std::process`` interface)
* Pseudo tty creation for child
* The ``unshare`` and ``setns``

The following is out of scope:

* mounting file systems
* setting up network
* in-container and out of container supervision
* handing child signals

.. _lithos: http://lithos.readthedocs.org
.. _vagga: http://vagga.readthedocs.org
