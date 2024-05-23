:orphan:

.. _copypath_action:

copyPath Action
===============

The ``copyPath`` action will copy a path locally on the system.

Supported parameters:

.. list-table::
    :widths: 6 7 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``sourcePath``
      - ``string`` - required.
      - A required string value representing the source path of the file or directory to copy.
    * - ``destPath``
      - ``string`` - required.
      - A string value representing the destination path of the file or directory to copy.
    * - ``recursive``
      - ``boolean`` - optional.
      - An optional boolean parameter that defaults to ``False``, indicating whether to recursively copy the source path.
        This generally needs to be set to ``True`` if you want to copy subdirectories of the source path directory.

        Essentially this will specify the ``-R`` flag to the Linux/UNIX ``cp`` command.
    * - ``update``
      - ``boolean`` - optional.
      - An optional boolean parameter (which defaults to ``False``) which when set to ``True`` will only copy the path when
        the source is newer than the destination.

        Essentially this will specify the ``-u`` flag to the Linux/UNIX ``cp`` command.
    