# mongo-connection-check

A downloadable executable that checks the connectivity from your client machine to a remote MongoDB database deployment. If a connection cannot be made, provides advice on how to diagnose and fix the connection issue. The type of MongoDB deployment checked can be a self-managed database on-prem/in-cloud or a database hosted in the [MongoDB Atlas](https://www.mongodb.com/cloud/atlas) cloud DBaaS.

![Screenshot of the mongo-connection-check tool](.tool_pic.png)

The checks this tool performs are based on the blog post [Some Tips for Diagnosing Client Connection Issues for MongoDB Atlas](http://pauldone.blogspot.com/2019/12/tips-for-atlas-connectivity.html).

## Downloads

 * [Linux](https://github.com/pkdone/mongo-connection-check/releases/download/0.8.1/mongo-connection-check) (x86-64)
 * [Windows 10](https://github.com/pkdone/mongo-connection-check/releases/download/0.8.1/mongo-connection-check.exe) (x86-64)
 * [Max OS X](https://github.com/pkdone/mongo-connection-check/releases/download/0.8.1/mongo-connection-check_mac) (x86-64) &nbsp;&nbsp;&nbsp;__NOTE:__ Rename this file once download, __removing the suffix '\_mac'__

These downloads are for the latest version of _mongo-connection-check_ (version __0.8.1__). For earlier versions, see this project's [releases page](https://github.com/pkdone/mongo-connection-check/releases)

## How To Run

Change the downloaded binary file's __permissions to be executable__ on your local OS - example terminal/shell command for Linux/Mac shown here:

```console
chmod u+x mongo-connection-check
```

__Open a terminal/prompt/shell and run the following__ _(in this example, to connect to a local MongoDB standalone server)_:

```console
./mongo-connection-check mongodb://localhost
```

&nbsp;&nbsp;__NOTE__: 
 * On __Windows__, first replace the text `./mongo-connection-check` with `mongo-connection-check.exe` in the command line shown above; you will also be prompted with some security dialog boxes to approve the safety of the executable
 * On __Mac OS X__, you will receive a prompt saying "Cannot be opened because the developer cannot be verified", therefore, if you trust this binary, you will then need to view the _Security & Privacy_ settings for the downloaded file and press the __Allow Anyway__ button, as shown below, before trying to run the command again:
 
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src=".mac_allow_access.png" width="370" height="317" alt="Screenshot of Allow Anyway option in Mac OS X"/>

### Further Help

To see the full __help__ information for this tool, include a `-h` parameter:

```console
./mongo-connection-check -h
```

__Example__ command line for attempting to connect to a remote MongoDB __Atlas__ deployment, using an example __SRV service name__ with a username & password embedded in the MongoDB URL:

```console
./mongo-connection-check "mongodb+srv://myusr:pswd@ctr.a1b2.mongodb.net/?retryWrites=true"
```

__Example__ command line for attempting to connect to a remote MongoDB __Atlas__ deployment, using an example SRV service name with the __username & password provided as parameters__ separate from the MongoDB URL:

```console
./mongo-connection-check -u myusr -p pswd "mongodb+srv://ctr.a1b2s.mongodb.net/?retryWrites=true"
```

__Example__ command line for attempting to connect to a remote __self-managed MongoDB cluster deployment__, using an example __seed list__ of individual server hostnames & ports in the MongoDB URL, with TLS enabled (username and password not specified in this example):

```console
./mongo-connection-check "mongodb://clstr1.acme.com:27017,clstr2.acme.net:27017/test?tls=true"
```

## Checks Performed

The tool will attempt to perform the following 7 checks sequentially, terminating as soon as one of the checks fails, and providing advice on how to then diagnose & fix the connectivity issue (if any):
 1. __URL-CHECK__. Confirms the URL contains a seed list of target server(s)/port(s) to try, or a DNS SRV service name.
 2. __MEMBERS-CHECK__. Determines the seed list of individual servers in the deployment (specifically if the URL defines a service name, looks up the SRV service in DNS to obtain the cluster's membership).
 3. __DNS-IP-CHECK__. Determines the IP addresses of each of the individual servers in the deployment, using DNS lookups.
 4. __SOCKET-CHECK__. Confirms a TCP socket connection can be established to one or more of the target servers in the seed list.
 5. __DRIVER-CHECK__. Confirms the MongoDB driver can validate the URL (including re-performing the SRV DNS resolution if required).
 6. __DBPING-CHECK__. Confirms the MongoDB driver can connect to the remote MongoDB deployment using the MongoDB 'dbping' command.
 7. __HEALTH-CHECK__. Retrieves the running deployment's type (e.g. standalone/replica-set/sharded-cluster/shared-atlas-tier) and, if the deployment is a replica set specifically, the primary & secondary member names and types.

## Supported MongoDB Deployments

Any MongoDB deployments of MongoDB versions 3.6+, if self managed, and MongoDB versions 4.2+, if hosted in Atlas, for any type of deployment topology, including:
 * A standalone single server
 * A replica set
 * A sharded cluster
 * An Atlas M0/M2/M5 shared tier _(which is fronted by a reverse proxy, under the covers)_
 
## Building The Project

_(ensure you've cloned/copied this GitHub project first to your local machine)_

 1. Install the latest version of the [Rust development environment](https://www.rust-lang.org/tools/install), if it isn't already installed, via the __rustup__ utility, including the _rustc_ compiler & the _cargo_ package/build manager. _NOTE:_ If building on Windows 10, first ensure you have Microsoft's [Build Tools for Visual Studio 2019](https://visualstudio.microsoft.com/thank-you-downloading-visual-studio/?sku=BuildTools&rel=16) installed (specifically when running that installer choose the _C++ build tools_ option)

 2. From a terminal/prompt/shell, from this project's root folder, run Rust's _cargo_ command to build the project and run the debug version of the tool executable, as shown in the example below (change the URL to match the specific MongoDB database deployment target you want to test):
 
```console
cargo build
cargo run -- "mongodb+srv://myusr:pswd@mycluster.a113z.mongodb.net"
```

 * _OPTIONAL_: Build a _production release_ version of the project's executable:
```console
cargo build --release
```
 
 * _OPTIONAL_: Run the project's _unit tests_:
```console
cargo test
```
 
 * _OPTIONAL_: Run the project's _lint_ checks (to catch common mistakes and suggest where code can be improved):
```console
cargo clippy
```

 * _OPTIONAL_: Highlight any lines of code which have a length greater than 100 characters:
```console
cat src/main.rs | awk 'length($0) > 100'
```

## Potential Future Enhancements

* __Perform ICMP Ping Test__. Add to part of the stage4 socket check stage to perform an ICMP PING test (sometimes a ping may not get through the firewall but a socket will, or vice versa, so if a socket test fails then try a ping and if that succeeds, at least make that information available, as it will help with any subsequent connectivity diagnosis).
* __Integration Tests__. Add integration tests to the project to reduce the risk of regressions.

