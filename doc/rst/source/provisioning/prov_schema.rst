
Provision Schema
================

Introduction
------------

Provisioning instructions are currently provided as .txt file of generic lists of key/value pairs representing parameters
(params) for the required resource wanting to be created. These generally map to abstractions or directly-related parameters
to the provisioning APIs of the various providers.
Different resource / provider combinations will require different key/values, and will support
different optional functionality.

General Parameters
------------------

``provider``
    A string representing the name of the provider to use: "linode", "vultr", etc.

``action``
    A string representing the action type / command to run. See the Actions section.

``waitType``
    A string representing the 'wait type' to use after performing the action, i.e. to wait for the resource
    to become available with an IP address.


Actions
=======

createInstance
--------------

Creates a cloud VPS instance with the specified provider, using the specified parameters, specific to that provider.



