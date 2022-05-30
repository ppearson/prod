
Provision Schema
================

Introduction
------------

Provisioning instructions are currently provided as .txt file of generic lists of key/value pairs representing parameters
(params) for the required resource wanting to be created. These generally map to abstractions or directly-related parameters
to the provisioning APIs of the various providers.
Different resource / provider combinations will require different key/values, and will support
different optional functionality.

For .txt file format, parameters must be specified one per line, with the name of the parameter being followed after the ``:``
character (and optional but recommended) space by the wanted value of the parameter.
The ``#`` character will mark lines as comments, which will be ignored.


General Parameters
------------------

``provider``
    A string representing the name of the provider to use: "linode", "vultr", "binary_lane", "digital_ocean", etc. This is a required parameter, and must
    be specified.

``action``
    A string representing the action type / command to run. See the Provision Actions section for the list of supported actions. This
    is a required parameter, and must be specified.

``waitType``
    A string representing the 'wait type' to use after performing the action, i.e. to wait for the resource
    to become available with an IP address.

    Valid values are:

    .. list-table::
        :widths: 5 10
        :header-rows: 1

        * - Value
          - Description
        * - returnImmediately
          - Returns immediately after performing the backing API request for the resource (i.e. creation of instance), and doesn't wait at all.
        * - waitForResourceCreation
          - Waits until the resource in question is *partially* available - at least in terms of known details. i.e. for creation
            of VPS instances, it will wait until an IP address of the instance is known.
        * - waitForResourceFinalised
          - Waits until the resource in question is fully ready to be used. This is the default if no wait type is specified.


Provision Actions
=================

createInstance
--------------

Creates a cloud VPS instance with the specified provider, using the specified parameters, specific to that provider.

**Linode**

.. list-table::
    :widths: 8 5 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``region``
      - Required
      - Region value, representing what region to use to create a cloud instance in.
    * - ``type``
      - Required
      - Type value, representing what type / size of instance to create.
    * - ``image``
      - Required
      - Image value, representing the OS image type to use to create the instance.
    * - ``root_pass``
      - Required
      - String value representing the root password to set for the root user account on the instance.
    * - ``label``
      - Optional
      - String value representing the label or name of the instance.
        **Note:** this needs to be unique for all instances that exist, otherwise the instance will not be
        able to be created.
  
Example recipe file:

.. code-block:: none

    # Deploy Linode node in Sydney
    provider: linode
    action: createInstance

    region: ap-southeast
    image: linode/debian11
    type: g6-nanode-1
    root_pass: o2t34svsg5de5hhd0b
    label: mysmallserver

**Vultr**

.. list-table::
    :widths: 8 5 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``region``
      - Required
      - Region value, representing what region to use to create a cloud instance in.
    * - ``plan``
      - Required
      - Plan value, representing what plan type / size of instance to create.
    * - ``os_id``
      - Required
      - OS ID value, representing the OS image type to use to create the instance.
    * - ``label``
      - Optional
      - String value representing the label or name of the instance.
    * - ``enable_ipv6``
      - Optional
      - Bool value indicating whether IPv6 should be enabled on the instance. Defaults to ``false``.
    * - ``backups``
      - Optional
      - Bool value indicating whether backups should be enabled on the instance. Defaults to ``false``.

Example recipe file:

.. code-block:: none

    # Create a Vultr $5 instance in Sydney running Debian 11
    provider: vultr
    action: createInstance

    plan: vc2-1c-1gb
    region: syd
    # debian 11 x64
    os_id: 477
  
**Binary Lane**

.. list-table::
    :widths: 8 5 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``region``
      - Required
      - Region value, representing what region to use to create a cloud instance in.
    * - ``size``
      - Required
      - Size value, representing what plan type / size of instance to create.
    * - ``image``
      - Required
      - Image value, representing the OS image type to use to create the instance.
    * - ``name``
      - Optional
      - String value representing the label or name of the instance.
    * - ``ipv6``
      - Optional
      - Bool value indicating whether IPv6 should be enabled on the instance. Defaults to ``false``.
  
Example recipe file:

.. code-block:: none

    # Create a Binary Lane $3.5 instance in Sydney running Debian 11
    provider: binary_lane
    action: createInstance

    region: syd
    name: server1
    image: 31
    size: std-min

**Digital Ocean**

.. list-table::
    :widths: 8 5 30
    :header-rows: 1
    :stub-columns: 1

    * - Parameter
      - Type
      - Description
    * - ``region``
      - Required
      - Region value, representing what region to use to create a cloud instance in.
    * - ``size``
      - Required
      - Size value, representing what plan type / size of instance to create.
    * - ``image``
      - Required
      - Image value, representing the OS image type to use to create the instance.
    * - ``name``
      - Required
      - String value representing the label or name of the instance.
    * - ``ipv6``
      - Optional
      - Bool value indicating whether IPv6 should be enabled on the instance. Defaults to ``false``.


