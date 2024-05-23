
Controlling General Schema
==========================

Introduction
------------

The general schema parameters are specified as base-level YAML parameters (within the first YAML document item) at the top
of the Control script YAML document.

General
-------
 
``provider``
    A string representing the name of the control provider to use, of which the only current ones supported are:
    ``linux_debian`` or ``linux_fedora``, with the ``linux_debian`` implementation being the most functional and tested currently.
    
    This is a required parameter, and must be specified.

``hostname``
    A string representing the hostname or IP address of the host machine to connect to in order to perform actions.
    If not specified, Prod will interactively request from the user the hostname to provide, as it's required information.
    Alternatively, a special string of ``$PROMPT`` can be specified, which will also cause Prod to interactively request from the user
    the hostname to connec to.

    Note: This parameter can also be specified with the name of ``host`` for backwards compatibility.

    Note: The hostname can optionally have a traditional port number specified after the ``:`` character, or it can be specified
    via the dedicated ```port``` parameter below.

``port``
    A number representing the SSH port to connect to if the default of 22 should not be used when connecting. Prod will default to using port ``22`` when 
    this is not specified.

``systemValidation``
    TODO.

``user``
    The username to use when connecting as a string. If this is not provided (and the authentication type is assumed to be username/password) then Prod will
    interactively prompt for the username to use to connect to the host machine. A special string of ``$PROMPT`` can also be specified, which will similarly
    prompt for the username to use.


Authentication
--------------

Prod supports both **Username/Password** SSH authentication to hosts as well as **Public/Private key** authentication, and needs to be configured
specifically to control each method via parameters.

``authType``
    A string representing the type of authentication to use, in the event when it's non-obvious to Prod what type to use.
    
    This is an optional parameter, and if not specified, Prod will attempt to work the authentication type to use out automatically
    based off the provided authentation parameters, but if conflicting parameters are provided for both types of authentication, this parameter
    can be helpful.
    
    The valid values for this parameter are: ``userpass`` for Username/Password authentication and ``publickey`` for public/private key
    authentication.

Username/Password Authentication
````````````````````````````````

For Username/Password authentication, the only additional parameter that can be specified is:

``password``
    The password to use when connecting as a string. If this is not provided (and the authentication type is assumed to be username/password) then Prod will
    interactively prompt for the password to use with the supplied username in order to connect to the host machine. Prod specifically will not show the password
    you enter as you type the password when interactively prompting for the password.
    
    A special string of ``$PROMPT`` can also be specified, which will similarly prompt for the password to use.

    **Note**: Saving passwords as plain-text in files should be done responsibly, as it is almost always a security risk and bad practice in general.

Public/Private key Authentication
`````````````````````````````````

For Public/Private key authentication, the following additional parameters can be specified:

``publicKeyPath``
    The file path to the SSH public key to use (it normally has a ``.pub`` extension) for connecting to the host via SSH.
    This is a required parameter if using Public/Private key authentication, and must be specified.

``privateKeyPath``
    The file path to the SSH private key to use for connecting to the host via SSH.
    This is a required parameter if using Public/Private key authentication, and must be specified.

``passphrase``
    The passphrase to use as a string when connecting via Public/Private key authentation.
    This is optional if your authentication keys do not require a passphrase.

    A special string of ``$PROMPT`` can also be specified, which will cause Prod to interactively prompt the for the passphrase to use before connecting,
    and Prod will specifically will not show the passphrase as you type it in this mode.

    **Note**: Saving passphrases as plain-text in files should be done responsibly, as it is almost always a security risk and bad practice in general.