
Controlling
===========

Introduction
------------

Controlling is the process of running basic commands over an SSH connection to configure servers. Currently this logic is in the form
of raw atomic 'actions', although in the future it is hoped there will be more robust 'configuration' functionality, which will try
and change configuration state to a designed target end state.

Current functionality exists to perform actions on Debian GNU/Linux platforms, although Prod's design should allow other platforms to
be supported.

