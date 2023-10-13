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

* Add verbose mode to control, which lists the actions that are being run on the remote machine.
* Implement and verify ssh key connection with ControlConnectionSSHrs

Larger-scale longer-term changes:

* Control scripts using other formats (.txt / properties?)
* State-based changes with final state verification, rather than current 'action' based changes with little error checking...
