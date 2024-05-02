
Prod Documentation
==================

Introduction
------------

Prod is a basic command line VPS :doc:`provisioning/index` and :doc:`controlling/index` (configuration / orchestration) tool,
partially intended as a vehicle to learn the Rust programming language with a new project, as well as to learn about HTTP web
services from VPS providers, although also to scratch an itch of making my own basic version of a VPS provisioning and configuration
tool, approximating some functionality of tools like Terraform and Ansible.

Prod's current functionality includes limited support for :doc:`provisioning/index` cloud VPS instances (with several providers supported
to a limited degree), as well as support for :doc:`controlling/index` the servers (running commands to configure them) afterwards,
based off YAML scripts describing actions and properties of what is desired.

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   provisioning/index

   controlling/index


