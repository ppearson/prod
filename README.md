Prod
====

Copyright 2021-2024 Peter Pearson.


Prod is a basic command line VPS provisioning and controlling (configuration / orchestration) tool,
partially intended as a vehicle to learn the Rust programming language with a new project, as well as to learn
about HTTP web services from VPS providers, although also to scratch an itch of making my own basic version of
a VPS provisioning and configuration tool, approximating some functionality of tools like Terraform and Ansible.

Prod's current functionality includes limited support for Provisioning cloud VPS instances (with several providers
supported to a limited degree), as well as support for Controlling the servers (running commands on them to
configure them) afterwards, based off YAML scripts describing actions and properties of what is desired.

It's still very much work-in-progress, although it is functional to a basic degree.


Control functionality can utilise either the `ssh2` crate (which depends on openssl) or the `ssh-rs` crate, and this can be
controlled with features in `Cargo.toml`, or disabled completely.


Provisioning
------------

Prod currently has limited provisioning support for creating and destroying VPS instances with the following 'providers':

* Binary Lane
* Vultr
* Linode
* Digital Ocean

Prod can also list available instance types, locations and OS images for the various providers.

In the near future the plan is to add support for creating other types of instances (high-perf compute, GPU, etc), as well as
provisioning block storage, in addition to DNS and network configuration from these providers where possible, and maybe other things.

Example Provision recipe file, which will create a $5 Vultr cloud instance in Sydney, running Debian Linux 11:

    # Create a Vultr $5 instance in Sydney running Debian 11
    provider: vultr
    action: createInstance

    plan: vc2-1c-1gb
    region: syd
    # debian 11 x64
    os_id: 477

To perform a provision, set the environment variable to control the private API key for the provider you want to use (Vultr in this example) - you
 will need to create your own for the respective provider:

    export PROD_VULTR_API_KEY=AN259_USE_YOUR_OWN_NYMK4HNKSFE5HEDEG3434T

and then run:

    ./prod provision ~/prod/examples/provision/vultr_create_instance_small_sydney.txt

which will eventually start the instance:

    Vultr instance created, id: 98ckefb8b-983f-859g-829503-68295ag ...
    Waiting for instance to spool up...
    Have instance IP: 144.33.109.42
    Waiting for server to finish install/setup...
    Cloud instance created successfully:

    id:             98ckefb8b-983f-859g-829503-68295ag
    ip:             144.33.109.42                       
    root_password:  mbk49nf9genk


Controlling
-----------

Controlling currently consists of running actions, such as adding new users, installing packages, copying files, editing files, adding firewall rules and starting services. Control access to servers/hosts is via SSH control streams, and both password and public/private key authentication methods
are supported. 

Control scripts are currently YAML files which control which actions to run, and the parameters for them. Debian and Fedora Linux action providers
are currently supported.

Below is an example control script which adds a new user, installs fail2ban, stops the fail2ban service, creates local copies of the fail2ban
config and jail files, edits the local copy, starts the fail2ban service, and then adds a new firewall rule.

    ---
    provider: linux_debian
    host: $PROMPT
    user: root
    actions:
    - addUser:
        username: MrUser
        password: DontForgetThePassword
        groups:
        - sudo
    - installPackages:
        packages:
        - "fail2ban"
    - systemCtl:
        service: "fail2ban"
        action: stop
    - copyPath:
        sourcePath: /etc/fail2ban/fail2ban.conf
        destPath: /etc/fail2ban/fail2ban.local
    - copyPath:
        sourcePath: /etc/fail2ban/jail.conf
        destPath: /etc/fail2ban/jail.local
    - editFile:
        backup: false
        filepath: "/etc/fail2ban/jail.local"
        insertLine:
          position: below
          matchString: '[sshd]'
          insertString: 'enabled: true'
          matchType: startsWith
          onceOnly: true
          reportFailure: false
        replaceLine:
          matchString: 'bantime  = 10m'
          replaceString: 'bantime  = 120m'
          matchType: startsWith
          onceOnly: true
          reportFailure: false
    - systemCtl:
        service: "fail2ban"
        action: start
    - systemCtl:
        service: "fail2ban"
        action: restart
    - firewall:
        type: ufw
        enabled: true
        rules:
        - "allow 80/tcp"

To run a control script, run:

    ./prod control <control_script_path.yaml>
