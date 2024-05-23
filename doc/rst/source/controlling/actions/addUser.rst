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
        
        **Note**: Saving passwords as plain-text in files should be done responsibly, as it is almost always a security risk and bad practice in general.
    * - ``createHome``
      - ``boolean`` - optional.
      - An optional boolean parameter that defaults to ``True``, indicating whether a ``$HOME`` directory should be created for the user.
    * - ``shell``
      - ``string`` - optional.
      - An optional string value that can be provided to override the default shell of ``/bin/bash`` which is used.
    * - ``group``
      - ``string`` - optional.
      - An optional string value that can be provided to indicate this new user needs to be added to this one group.
    * - ``groups``
      - ``string array`` - optional.
      - An optional array of string values that can be provided to indicate this new user needs to be added to these specified groups.