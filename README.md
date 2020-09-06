# mongo-connection-check

A downloadable executable that checks the connectivity from your client machine to a remote MongoDB database deployment, If a connection cannot be made, provides advice on how to diagnose and potentially fix the connection issue. The MongoDB deployment tested can be a self-managed database on-prem/in-cloud or a hosted in the [MongoDB Atlas](https://www.mongodb.com/cloud/atlas) DBaaS in the cloud.

![Screenshot of the mongo-connection-check tool](.tool_pic.png)

Attempts to perform the following 7 checks sequentially, terminating as soon as one of the checks fails (if any), and providing advice on how to then diagnose & potentially fix the connectivity issue:
 1. __URL-CHECK__. Confirms the URL contains a seed list of target server(s)/port(s) to try, or a DNS SRV service name.
 2. __MEMBERS-CHECK__. Determines the seed list of individual servers in the deployment (specifically if the URL defines a service name, looks up the SRV service in DNS to obtain the cluster's server member addresses and ports).
 3. __DNS-IP-CHECK__. Determines the IP addresses of each of the individual servers in the deployment, via DNS lookups.
 4. __SOCKET-CHECK__. Confirms a TCP socket connection can be established to one or more of the target servers in the seed list/
 5. __DRIVER-CHECK__. Confirms the MongoDB driver can validate the URL (including re-performing the SRV DNS resolution if required).
 6. __DBPING-CHECK__. Confirms the MongoDB driver can connect to the remote MongoDB deployment using the MongoDB 'dbping' command.
 7. __HEALTH-CHECK__. Retrieves the running deployment's type (e.g. standalone/replica-set/sharded-cluster/shared-atlas-tier) and, if the deployment is a replica set specifically, the primary & secondary members of the replica set.

## Supported MongoDB Deployments

Any MongoDB deployments of MongoDB versions 3.6+, if self managed, and MongoDB versions 4.2+, if hosted in Atlas, for any type of deployment topology, including:
 * A standalone single server
 * A replica set
 * A sharded cluster
 * An Atlas M0/M2/M5 shared tier _(which is fronted by a reverse proxy, under the covers)_
 
## Biary Executable Downloads

 * Linux (x86-64) TODO; link
 * Windows 10 (x96-64) TODO; link
 * Max OS X TODO (x86-64) TODO; link

These download links are for the latest version of _mongo-connection-check_ (version TODO). For earlier versions, see this project's releases page TODO: link.

TODO: mention any other dependences

## How To Run

Open a terminal/shell and run the following (in this example to connect to a local MongoDB standalone server):

```console
./mongo-connection-check mongodb://localhost
```

To see the full __help__ information for this tool, include a `-h` parameter, for example:

```console
./mongo-connection-check -h
```

__Example__ command line for attempting to connect to a remote MongoDB __Atlas__ deployment, using an example __SRV service name__ with a username & password embedded in the MongoDB URL:

```console
./mongo-connection-check "mongodb+srv://myusr:pswd@ctr.a1b2.mongodb.net/?retryWrites=true"
```

__Example__ command line for attempting to connect to a remote MongoDB __Atlas__ deployment, using an example SRV service name with a __username & password provided as parameters__ separate from the MongoDB URL:

```console
./mongo-connection-check -u mysyr -p pswd "mongodb+srv://ctr.a1b2s.mongodb.net/?retryWrites=true"
```

__Example__ command line for attempting to connect to a remote __self-managed MongoDB cluster deployment__, using an example __seed list__ of individual server hostnames & ports in the MongoDB URL, with TLS enabled (username and password not specified in this example):

```console
./mongo-connection-check "mongodb://clstr1.acme.com:27017,clstr2.acme.net:27017/test?tls=true"
```

## How to Build The Project

_(ensure you've cloned/copied this GitHub project first to your local machine)_

 1. Instal the latest version of [Rust is installed](https://www.rust-lang.org/tools/install), if it isn't already, via the __rustup__ utility, including the _rustc_ compiler & the _cargo_ package/build manager

 2. From a terminal/shell, change directory to this project's root folder and then run Rust's _cargo_ command to build the project and run the debug version of the tool executable, as shown in the example below (change the URL to match your MongoDB database deployment target):
 
```console
cargo build
cargo run -- "mongodb+srv://main_user:Password1@testcluster.s703u.mongodb.net/"
```

 * _OPTIONAL_: Run the project's _unit tests_:
```console
cargo test
```
 
 * _OPTIONAL_: Run the project's _lint_ checks (to catch common mistakes and suggest where code can be improved):
```console
cargo clippy
```

 * _OPTIONAL_: Highlight any code lines which have a length greater than 100 characters:
```console
cat src/main.rs | awk 'length($0) > 100'
```

## Potential TODOs For The Future

* __Concurrent Socket Tests__. Change the stage4 check stage to use an async socket API and then concurrently peform a socket test for each member of the server (with the 4 second timeout configured for this tool, this should mean that for 3 cluster members, the total time taken will be just over 4 seconds rather than around 12 seconds, that would currently be incurred, for example).
* __Perform ICMP Ping Test__. Change part of the stage4 socket check stage to perform an ICMP PING test (sometimes a ping may not get through the firewall but a socket will, or vice versa, so if a socket test fails then try a ping and if that succeeds, at least make that information available, as it will help with any subsequent connectivity diagnosis).
* __Integration Tests__. Add integration tests to the project to reduce the risk of regressions.

