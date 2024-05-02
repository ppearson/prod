
Provisioning
============

Introduction
------------

Provisioning is the process of creating or modifying resources: in Prod's case, this generally means VPS cloud servers from
providers such as Vultr, Linode, Binary Lane or Digital Ocean.

.. toctree::
   :maxdepth: 2

   self
   prov_schema

Provision Provider Support
--------------------------

Below is a basic overview of what's currently supported for the four direct-API providers currently, as well as the
more generic OpenStack provider which several other providers provide APIs for.

+---------------------------------+---------+---------+---------------+-------------+-----------+
| Feature                         | Vultr   | Linode  | Digital Ocean | Binary Lane | OpenStack |
+=================================+=========+=========+===============+=============+===========+
| List Regions / Locations        | |tick|  | |tick|  | |tick|        | |tick|      | |cross|   |
+---------------------------------+---------+---------+---------------+-------------+-----------+
| List VPS Types / Sizes          | |tick|  | |tick|  | |tick|        | |tick|      | |cross|   |
+---------------------------------+---------+---------+---------------+-------------+-----------+
| List Operating Systems / Images | |tick|  | |tick|  | |tick|        | |tick|      | |cross|   |
+---------------------------------+---------+---------+---------------+-------------+-----------+
| Create Cloud Instance           | |tick|  | |tick|  | |tick|        | |tick|      | |cross|   |
+---------------------------------+---------+---------+---------------+-------------+-----------+
| Delete Cloud Instance           | |tick|  | |tick|  | |tick|        | |tick|      | |cross|   |
+---------------------------------+---------+---------+---------------+-------------+-----------+

Using Providers
---------------

In order to use providers (internally done in Prod via providers' individual HTTP web APIs) to provision resources, you must have
an "API Key" created for your account. For Digital Ocean, this key (API Token) also is required for listing available resources
(like locations).

You can create these keys in the account settings section of the web interface of each provider, and you will need to set an
environment variable in order for Prod to know about it and use it for its requests to the respective API endpoint of the provider.

**Vultr**
    To configure the Vultr provider infrastructure, you must set the ``$PROD_VULTR_API_KEY`` environment variable to the value of your
    Vultr API key you created in the Vultr web interface for your account. You may also have to allow access to allowed IP ranges
    in order for the API to be used from the machine you will be running it on.

**Linode**
    To configure the Linode provider infrastructure, you must set the ``$PROD_LINODE_API_KEY`` environment variable to the value of your
    Linode API key you created in the Linode web interface for your account.

**Digital Ocean**
    To configure the Digital Ocean provider infrastructure, you must set the ``$PROD_DIGITAL_OCEAN_API_TOKEN`` environment variable to the
    value of your Digital Ocean API token you created in the Digital Ocean web interface for your account. Note: Digital Ocean requires
    the token to be configured even for listing available regions and droplet types/sizes.

**Binary Lane**
    To configure the Binary Lane provider infrastructure, you must set the ``$PROD_BINARY_LANE_API_TOKEN`` environment variable to the
    value of your Binary Lane API key you created in the Binary Lane web interface for your account. Note: listing available OS images
    requires that the token be configured, but listing regions and sizes doesn't.

See :doc:`prov_schema` for details on how to specify Provision instructions and parameters for Prod for each Provider implementation.


Running Prod
------------

To run a provision file with prod, use the following command line syntax (after ensuring that the respective API key is set
if required):

``prod provision <path_to_provision_file.txt>``

as an example, running the following example recipe file for Vultr (after setting up a Vultr API Key and setting the $PROD_VULTR_API_KEY env variable):

``prod provision examples/provision/vultr_create_instance_small_sydney.txt``

Will create a $5 Vultr instance with 25 GB of storage in Sydney, producing this output to the terminal when the instance has
finished provisioning:

.. code-block:: none

    Vultr instance created, id: 56f75a46-2ea1-2c23-51b5-d33ab4e16a42 ...
    Waiting for instance to spool up...
    Have instance IP: 104.32.54.22
    Waiting for server to finish install/setup...
    Cloud instance created successfully:
    
    id:             56f75a46-2ea1-2c23-51b5-d33ab4e16a42
    ip:             104.32.54.22                       
    root_password:  SH}Rjrqeg}4tp34hrtheff



.. |tick|    unicode:: U+2714
.. |cross|   unicode:: U+2718