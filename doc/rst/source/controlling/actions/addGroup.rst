:orphan:

.. _addgroup_action:

addGroup Action
===============

The ``addGroup`` action will add a new group to the system.

Supported parameters:

.. list-table::
    :widths: 6 7 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``name``
      - ``string`` - required.
      - A required string value representing the name of the new group to add.
    * - ``user``
      - ``string`` - optional.
      - An optional string value representing a single existing user to add to the newly-created group.
    * - ``users``
      - ``string array`` - optional.
      - An optional string array parameter representing multiple users to add to the newly-created group.
    