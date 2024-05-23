:orphan:

.. _disableswap_action:

disableSwap Action
==================

The ``disableSwap`` action allows disabling and removing Swap mountpoints on the system and their backing file, which can be useful to maximise available space on some VPSs.

Supported parameters:

.. list-table::
    :widths: 6 7 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``filename``
      - ``string`` - required.
      - A required string parameter representing the backing filepath of the swap mountpoint / filesystem on the system to be disabled and then deleted.

        This can either be an absolute path to the backing file on disk, or the value of ``*`` can be used to indicate Prod should disable and delete all found active Swap mountpoint
        systems and their backing files.