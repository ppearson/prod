
Version --next
--------------

* Added new 'configureSSH' control action, for easily configuring sshd configuration with common changes.
* Added new 'addPackageRepo' control action, allowing the ability to add additional custom package repositories for Debian,
  currently only for custom web URL definition downloads.

Version 0.3.1
-------------

* Added new 'createSystemdService' control action, which allows creating basic systemd service units and starting them.
* Improved convenience of editFile control action when specifying duplicate modification items, and provided better examples.
