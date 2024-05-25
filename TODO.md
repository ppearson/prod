TODO
====

Misc
----

* Add ability to do provisioning and then control afterwards in a single command invocation.

Provisioning
------------

* Provisioning from other file formats (YAML?)


Control
-------

Smaller things:

* Pre-validation of action parameters, before actually starting to connect and run actions.
* Better addUser support for setting user passwords and validating that the password has been set correctly.
* Better support for running as non-root user - i.e. using sudo.

Larger-scale longer-term changes:

* Support for running Control scripts on multiple hosts at once.
* Control scripts using other additional formats (.txt / TOML / properties?).
* State-based "target" changes with final state verification - i.e. "Idempotency", rather than current 'action'
  based changes with somewhat limited error checking...
