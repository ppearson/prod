:orphan:

.. _createsymlink_action:

createSymlink Action
====================

The ``createSymlink`` action will create a new named symlink on the remote system, pointing at a particular path location.

Supported parameters:

.. list-table::
    :widths: 6 7 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``linkPath``
      - ``string`` - required.
      - A required string parameter representing the path of the symlink to create (which will point elsewhere).
    * - ``targetPath``
      - ``string`` - optional.
      - A required string parameter which is the target path the created symlink should point to. This can be either a relative path or a full absolute path.
