:orphan:

.. _adduser_action:

addUser Action
==============

The ``addUser`` action will add a new user to the system.

Supported parameters:

.. list-table::
    :widths: 6 7 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``username``
      - ``string`` - required.
      - A required string value representing the name of the new user to add.
    * - ``password``
      - ``string`` - required.
      - A string value representing the password of the new user being added.
        
        This can be the string ``$PROMPT`` to cause Prod to interactively prompt the user for the password to use.

        The password value must meet the remote system's password complexity requirements or it will currently fail silently.
        
        **Note**: Saving passwords as plain-text in files should be done responsibly, as it is almost always a security risk and bad practice in general.
    * - ``createHome``
      - ``boolean`` - optional.
      - An optional boolean parameter that defaults to ``True``, indicating whether a ``$HOME`` directory should be created for the user.
    * - ``shell``
      - ``string`` - optional.
      - An optional string value that can be provided to override the default shell. If not specified, then Prod will default to using ``/bin/bash`` as the default,
        however specifying an empty string for this parameter will use the default shell configured for new users on the system (depending on the distribution,
        probably ``/bin/sh``).
    * - ``defaultGroup``
      - ``string`` - optional.
      - An optional string value that can be provided to indicate this new user needs to be added to this one (existing) group instead of having the default group be
        created based off the user name and the user added to that one.
    * - ``extraGroups``
      - ``string array`` - optional.
      - An optional array of string values that can be provided to indicate this new user needs to be added to these specified groups (that must exist already).

Example Snippets
----------------

Add a new user with the name of ``test1``, interactively prompting for the password to set, and also add the user to the ``sudo`` group:

.. code-block:: yaml

  actions:
  - addUser:
      username: test1
      password: $PROMPT
      extraGroups:
        - sudo

Add a new user with the name of ``test2``, interactively prompting for the password to set, setting the default shell to be ``/bin/zsh``, setting the default group of the
new user to ``test`` and also adding the user to the ``sudo`` group:

.. code-block:: yaml

  actions:
  - addUser:
      username: test2
      password: $PROMPT
      shell: /bin/zsh
      defaultGroup: test
      extraGroups:
        - sudo

Add a new user with the name of ``user4``, setting the password to be ``newpassword2``, and also add the user to the ``tempusers`` group, while not creating a new home directory for this user:

.. code-block:: yaml

  actions:
  - addUser:
      username: user4
      password: newpassword2
      createHome: False
      extraGroups:
        - tempusers