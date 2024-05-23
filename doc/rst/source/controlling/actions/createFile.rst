:orphan:

.. _createfile_action:

createFile Action
=================

The ``createFile`` action will create a new file on the system, optionally with the described text contents.

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
    * - ``content``
      - ``string`` - optional.
      - An optional string parameter which describes the text (can be multi-line) content to be inserted into the newly created file.
    * - ``permissions``
      - ``string`` - optional.
      - An optional string parameter representing any permission value to set for the newly-created file.
    * - ``owner``
      - ``string`` - optional.
      - An optional string parameter representing any owner user (which must exist as a user already) to set for the newly-created file.
    * - ``group``
      - ``string`` - optional.
      - An optional string parameter representing any group (which must exist as a group already) to set for the newly-created file.
    