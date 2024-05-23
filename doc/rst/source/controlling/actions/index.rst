
Control Actions Schema
======================

Introduction
------------

This section describes the supported Actions and their parameters that can be specified in a Control script, which
perform "actions", or config/changes to the system.

Actions list:

.. list-table::
    :widths: 8 30
    :header-rows: 1
    :stub-columns: 1

    * - Action
      - Description
    * - :ref:`addGroup <addgroup_action>`
      - Adds a new Group to the system.
    * - :ref:`addUser <adduser_action>`
      - Adds a new User to the system, providing control over the password, the default shell, user groups to set and more.
    * - :ref:`copyPath <copypath_action>`
      - Locally copies a path (filename/dir) on the remote system.
    * - :ref:`createDirectory <createdirectory_action>`
      - Creates a directory on the remote system.
    * - :ref:`createFile <createfile_action>`
      - Creates a file on the remote system, optionally with specified text contents.
    * - :ref:`createSymlink <createsymlink_action>`
      - Creates a symlink on the remote system pointing at a specified target path.
    * - :ref:`disableSwap <disableswap_action>`
      - Disables a specified swap mountpoint (or optionally all active ones) and deletes its backing file on disk.
    