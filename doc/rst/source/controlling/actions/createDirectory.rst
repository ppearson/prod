:orphan:

.. _createdirectory_action:

createDirectory Action
======================

The ``createDirectory`` action will create a new directory path on the system.

Supported parameters:

.. list-table::
    :widths: 6 7 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``path``
      - ``string`` - required.
      - A required string value representing the target path of the directory to create.
    * - ``multiLevel``
      - ``boolean`` - optional.
      - An optional boolean parameter that defaults to ``False``, indicating whether to create multiple levels of directories
        if needed, rather than just a single level. This can be useful when wanting to create a hierarchy of multiple directories in one Action.
    * - ``permissions``
      - ``string`` - optional.
      - An optional string parameter representing any permission value to set for the newly-created directory.
    * - ``owner``
      - ``string`` - optional.
      - An optional string parameter representing any owner user (which must exist as a user already) to set for the newly-created directory.
    * - ``group``
      - ``string`` - optional.
      - An optional string parameter representing any group (which must exist as a group already) to set for the newly-created directory.
    