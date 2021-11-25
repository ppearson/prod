Prod
====

Prod is a basic command line VPS provisioning and controlling (configuration / orchestration) tool,
partially intended as a vehicle to learn the Rust programming language with a new project, as well as to learn
about HTTP web services from VPS providers, although also to scratch an itch of making my own basic version of
a VPS provisioning and configuration tool, approximating some functionality of tools like Terraform and Ansible.

Prod's current functionality includes limited support for provisioning cloud VPS instances (with several providers
supported to a limited degree), as well as support for controlling the servers (running commands to configure them)
afterwards, based off text / YAML scripts describing properties of what is desired.

