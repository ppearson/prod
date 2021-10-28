
Provision
=========

Introduction
------------

Provision is the process of creating or modifying resources: in Prod's case, this generally means VPS cloud servers from
providers such as Linode, Vultr or Digital Ocean.

Provision Provider Support
--------------------------

Below is a basic overview of what's currently supported for the three providers currently.

+---------------------------------+---------+---------+---------------+
| Feature                         | Vultr   | Linode  | Digital Ocean |
+=================================+=========+=========+===============+
| List Locations                  | |tick|  | |tick|  | |tick|        |
+---------------------------------+---------+---------+---------------+
| List VPS Types                  | |tick|  | |tick|  | |tick|        |
+---------------------------------+---------+---------+---------------+
| List Operating Systems          | |tick|  | |tick|  | |tick|        |
+---------------------------------+---------+---------+---------------+
| Create Cloud Instance           | |tick|  | |tick|  | |cross|       |
+---------------------------------+---------+---------+---------------+
| Delete Cloud Instance           | |cross| | |cross| | |cross|       |
+---------------------------------+---------+---------+---------------+

Using Providers
---------------

In order to use providers (internally done in Prod via their HTTP web APIs) to provision resources, you must have an "API Key"
created for your account. For Digital Ocean, this key also is required for listing available resources (like locations).

You can create these keys in the account settings section of the web interface of each provider, and you will need to set an
environment variable in order for Prod to know about it and use it for its requests to the respective API endpoint of the provider.

**Linode**
    To configure the Linode provider infrastructure, you must set the ``$LINODE_API_KEY`` environment variable to the value of your
    Linode API key you created in the Linode web interface for your account.

**Vultr**
    To configure the Vultr provider infrastructure, you must set the ``$VULTR_API_KEY`` environment variable to the value of your
    Vultr API key you created in the Vultr web interface for your account. You may also have to allow access to allowed IP ranges
    in order for the API to be used from the machine you will be running it on.


See :doc:`prov_schema` for details on how to specify Provision instructions for Prod.


.. |tick|    unicode:: U+2714
.. |cross|   unicode:: U+2718