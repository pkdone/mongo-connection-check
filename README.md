# mongo-connection-check

A downloadable binary tool that checks the connectivity from your machine to a remote MongoDB deployment, and if a connection cannot be made, provides advice on how to diagnose and potentially fix.
. The MongoDB deployment tested could be self-managed database on-prem/in-cloud or hosted in the [MongoDB Atlas](https://www.mongodb.com/cloud/atlas) DBaaS in the cloud.

![Screenshot of mongo-connection-check](.tool_pic.png)

Attempts to perform the following 7 checks sequentially, terminating as soon as one of the checks fails (if any), providing advice on how to then diagnose and potentially fix the connectivity issue:
 1. __URL-CHECK__. Confirms URL contains a seed list of target server(s)/port(s) or a DNS SRV service name.
 2. __MEMBERS-CHECK__. Determines the seed list of individual servers in the deployment (specifically if the URL defines a service name, looks up SRV service in DNS to get the constituent cluster server member addresses and ports).
 3. __DNS-IP-CHECK__. Determines the IP addresses of each the individual servers in the deployment, via DNS lookups.
 4. __SOCKET-CHECK__. Confirms a TCP socket connection can be established to one or more the target servers in the seed list/
 5. __DRIVER-CHECK__. Confirms MongoDb driver can validate the URL (including SRV DNS resolution if required).
 6. __DBPING-CHECK__. Confirms the MongoDB driver can connect to the remote MongoDB deployment using the MongoDB 'dbping' command.
 7. __HEALTH-CHECK__. Retrieves the running deployment's type (e.g. standalone/replica-set/sharded-cluster/shared-atlas-tier) and the primary & secondary members if the deployment is a replica set.

## Supported Environments

Support client machine operating systems for running this tool:
 * Linux (x86-64)
 * Windows 10 (x96-64)
 * Max OS X TODO (x86-64_

Supported target MongoDB deployments include MongoDB versions 3.6 or greater, when self managedm and MongoDB version 4.2 or greater, when hosted in Atlas. All MongoDB deployment topologies are supported for checking, including: 
 * Standalone single server
 * Replica Set
 * Sharded Cluster
 * Atlas M0/M2/M5 shared tier (fronted by a reverse proxy)
 
## Where To Download

TODO link to github releases + mention of any other dependencies

## How To Run

Open a terminal/shell and run the following (in this example to connect to a local MongoDB standalone server):

```console
./mongo-connection-check mongodb://localhost
```

To see full help usage, include a `-h` parameter, for example:

```console
./mongo-connection-check -h
```

Example for attempting to connect to a remote MongoDB Atlas deployment, using a SRV service name with a username & password embedded in the MongoDB URL:

```console
./mongo-connection-check "mongodb+srv://myusr:pswd@ctr.a1b2.mongodb.net/?retryWrites=true"
```

Example for attempting to connect to a remote MongoDB Atlas deployment, using a SRV service name with a username & password provided as parameters separate from the MongoDB URL:

```console
./mongo-connection-check -u mysyr -p pswd "mongodb+srv://ctr.a1b2s.mongodb.net/?retryWrites=true"
```

Example for attempting to connect to a remote self-managed MongoDB cluster deployment, using a seed list of individual server hostnames & ports in the MongoDB URL, with TLS enabled (username and password not specified in this example):

```console
./mongo-connection-check "mongodb://clstr1.acme.com:27017,clstr2.acme.net:27017/test?tls=true"
```

## How to Build

TODO (inc dependencies)

TODO run test, clippy and line length check

## Potential TODOs For The Future

* TODO (add todos from other doc

