# mongo-connection-check

A downloadable executable that checks the connectivity from your client machine to a remote MongoDB database deployment, If a connection cannot be made, provides advice on how to diagnose and potentially fix. The MongoDB deployment tested could be a self-managed database on-prem/in-cloud or a hosted in the [MongoDB Atlas](https://www.mongodb.com/cloud/atlas) DBaaS in the cloud.

![Screenshot of the mongo-connection-check tool](.tool_pic.png)

Attempts to perform the following 7 checks sequentially, terminating as soon as one of the checks fails (if any), providing advice on how to then diagnose and potentially fix the connectivity issue:
 1. __URL-CHECK__. Confirms URL contains a seed list of target server(s)/port(s) or a DNS SRV service name.
 2. __MEMBERS-CHECK__. Determines the seed list of individual servers in the deployment (specifically if the URL defines a service name, looks up SRV service in DNS to get the constituent cluster server member addresses and ports).
 3. __DNS-IP-CHECK__. Determines the IP addresses of each the individual servers in the deployment, via DNS lookups.
 4. __SOCKET-CHECK__. Confirms a TCP socket connection can be established to one or more the target servers in the seed list/
 5. __DRIVER-CHECK__. Confirms MongoDb driver can validate the URL (including SRV DNS resolution if required).
 6. __DBPING-CHECK__. Confirms the MongoDB driver can connect to the remote MongoDB deployment using the MongoDB 'dbping' command.
 7. __HEALTH-CHECK__. Retrieves the running deployment's type (e.g. standalone/replica-set/sharded-cluster/shared-atlas-tier) and the primary & secondary members if the deployment is a replica set.

## Supported MongoDB Deployments

Any MongoDB deployments of MongoDB versions 3.6+ if self managed, and MongoDB versions 4.2+ if hosted in Atlas, for any type of cluster tolology, including:
 * Standalone single server
 * Replica set
 * Sharded cluster
 * Atlas M0/M2/M5 shared tier (fronted by a reverse proxy)
 
## Where To Download

 * Linux (x86-64) TODO; link
 * Windows 10 (x96-64) TODO; link
 * Max OS X TODO (x86-64) TODO; link

These links are for the latest version (version TODO). For ealier versions, see this project's releases page TODO: link.

TODO: mention any other dependences

## How To Run

Open a terminal/shell and run the following (in this example to connect to a local MongoDB standalone server):

```console
./mongo-connection-check mongodb://localhost
```

To see full __help)) usage, include a `-h` parameter, for example:

```console
./mongo-connection-check -h
```

__Example__ for attempting to connect to a remote MongoDB __Atlas__ deployment, using a __SRV service name__ with a username & password embedded in the MongoDB URL:

```console
./mongo-connection-check "mongodb+srv://myusr:pswd@ctr.a1b2.mongodb.net/?retryWrites=true"
```

__Example__ for attempting to connect to a remote MongoDB __Atlas__ deployment, using a SRV service name with a __username & password provided as parameters__ separate from the MongoDB URL:

```console
./mongo-connection-check -u mysyr -p pswd "mongodb+srv://ctr.a1b2s.mongodb.net/?retryWrites=true"
```

__Example__ for attempting to connect to a remote __self-managed MongoDB cluster deployment__, using a __seed list__ of individual server hostnames & ports in the MongoDB URL, with TLS enabled (username and password not specified in this example):

```console
./mongo-connection-check "mongodb://clstr1.acme.com:27017,clstr2.acme.net:27017/test?tls=true"
```

## How to Build

 1. Ensure you have the latest version of [Rust installed](https://www.rust-lang.org/tools/install) via the __rustup__ utility, including the _rustc_ compiler & the _cargo_ package/build manager

 2. From a terminal/shell, change directory to this project's root folder and then run the _Rust_ command to build the project and run the debug version of the tool executable, as shown in the example below (change the URL to match your MongoDB database deployment target):
 
```console
cargo build
cargo run -- "mongodb+srv://main_user:Password1@testcluster.s703u.mongodb.net/"
```

 * OPTIONAL: Run the project's _unit tests_:
```console
cargo test
```
 
 * OPTIONAL: Run the project's _lint_ checks (to catch common mistakes and suggest where code can be improved):
```console
cargo clippy
```

 * OPTIONAL: Highlight any code lines which have a length greater than 100 characters:
```console
cat src/main.rs | awk 'length($0) > 100'
```

## Potential TODOs For The Future

* Change the stage4 check to use an async socket API and then concurrently socket test each member server (with the 4 second timeout configured for this tool this means that for 3 cluster members, the total time taken will be just over 4 seconds rather than around 12 seconds that would be currently incurred, for example)
* Change part of the stage4 socket check perform an ICMP PING test (sometimes a ping may not get through the firewall but a socket will, or vice versa, so if a socket test fails then try a ping and if that succeeds, at least make that information available as it will help with any diagnosis)
* Add integration tests to the project

