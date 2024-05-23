
Controlling
===========

Introduction
------------

Controlling is the process of running basic commands over an SSH connection to configure servers. Currently this logic is in the form
of raw atomic Control 'actions', although in the future it is hoped there will be more robust 'configuration' functionality, which will
try and change configuration state to a designed target end state.

Authentication over SSH can be done via username/password authentication or by pub/private key authentication.

Current functionality exists to perform actions on Debian GNU/Linux (with stub implementations for the Fedora GNU/Linux platform, which
used to work to a degree, but hasn't been tested recently), although Prod's design would allow other platforms to be supported.

Control scripts are currently specified as .yaml documents, with 'general' parameters specifying things like the Control
provider name, the hostname to connect to, the authentication type and respective 'parameters' for that authentication type,
as well as an optional System Validation step, followed then by hierarchical descriptions of Actions/commands and their parameters.

General Control Specification Schema
------------------------------------

See the full documentation on :doc:`control_general_schema`.

Actions Specification Schema
----------------------------

See the full documentation on :doc:`actions/index`.