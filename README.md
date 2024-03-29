# mongo-connection-check

A downloadable executable that checks the connectivity from your client machine to a remote MongoDB database deployment. If a connection cannot be made, provides advice on how to diagnose and fix the connection issue. The type of MongoDB deployment checked can be a self-managed database on-prem/in-cloud or a database hosted in the [MongoDB Atlas](https://www.mongodb.com/cloud/atlas) cloud DBaaS.

![Screenshot of the mongo-connection-check tool](.tool_pic.png)

The checks this tool performs are based on the blog post [Some Tips for Diagnosing Client Connection Issues for MongoDB Atlas](http://pauldone.blogspot.com/2019/12/tips-for-atlas-connectivity.html).

## Downloads

 * [Linux](https://github.com/pkdone/mongo-connection-check/releases/download/1.2.1/mongo-connection-check-linux-x86_64-121)
 * [Windows 10](https://github.com/pkdone/mongo-connection-check/releases/download/1.2.1/mongo-connection-check-windows-x86_64-121.exe)
 * [Max OS X](https://github.com/pkdone/mongo-connection-check/releases/download/1.2.1/mongo-connection-check-macos-x86_64-121)

_NOTE:_ __Rename__ the file once downloaded, removing the last part of the name, to just be called __mongo-connection-check__ (or __mongo-connection-check.exe__ on Windows)

These downloads are for the latest version of _mongo-connection-check_ (version __1.2.1__). For earlier versions, see this project's [releases page](https://github.com/pkdone/mongo-connection-check/releases)

## How To Run

Change the downloaded binary file's __permissions to be executable__ on your local OS - example terminal/shell command for Linux/Mac shown here:

```console
chmod u+x mongo-connection-check
```

From a terminal/prompt/shell, __execute the tool__ by running the following _(in this example, to connect to a local MongoDB standalone server)_:

```console
./mongo-connection-check mongodb://localhost
```

&nbsp;&nbsp;__NOTE__: 
 * On __Windows__, first replace the text `./mongo-connection-check` with `mongo-connection-check.exe` in the command line shown above; when first attempting to run you will also be prompted with some security dialog boxes to approve the safety of the executable
 * On __Mac OS X__, you will receive a prompt saying __"Cannot be opened because the developer cannot be verified"__, therefore, if you trust this binary, you will then need to view the __Security & Privacy__ settings for the downloaded file and press the __Allow Anyway__ button, as shown below, before trying to run the command again:
 
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src=".mac_allow_access.png" width="370" height="317" alt="Screenshot of Allow Anyway option in Mac OS X"/>

### Further Help

To see the full __help__ information for this tool, include a `-h` parameter:

```console
./mongo-connection-check -h
```

__Example__ command line for attempting to connect to a remote MongoDB __Atlas__ deployment, using an example __SRV service name__ with a username & password embedded in the MongoDB URL (use double quotes rather than single quotes on Windows):

```console
./mongo-connection-check 'mongodb+srv://myusr:pswd@ctr.a1b2.mongodb.net/?retryWrites=true'
```

__Example__ command line for attempting to connect to a remote MongoDB __Atlas__ deployment, using an example SRV service name with the __username & password provided as parameters__ separate from the MongoDB URL (use double quotes rather than single quotes on Windows):

```console
./mongo-connection-check -u myusr -p pswd 'mongodb+srv://ctr.a1b2s.mongodb.net/?retryWrites=true'
```

__Example__ command line for attempting to connect to a remote __self-managed MongoDB cluster deployment__, using an example __seed list__ of individual server hostnames & ports in the MongoDB URL, with TLS enabled (username and password not specified in this example; use double quotes rather than single quotes on Windows):

```console
./mongo-connection-check 'mongodb://clstr1.acme.com:27017,clstr2.acme.net:27017/test?tls=true'
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

 1. Install the latest version of the [Rust development environment](https://www.rust-lang.org/tools/install), if it isn't already installed, via the __rustup__ utility, including the _rustc_ compiler & the _cargo_ package/build manager. _NOTE:_ If building on Windows 10, first ensure you have Microsoft's [Build Tools for Visual Studio 2019](https://visualstudio.microsoft.com/thank-you-downloading-visual-studio/?sku=BuildTools&rel=16) installed (and importantly, when running Microsoft's _build tools_ installer, choose the _C++ build tools_ option)

 2. From a terminal/prompt/shell, from this project's root folder, run Rust's _cargo_ command to build the project and run the debug version of the tool executable, as shown in the example below (change the URL to match the specific MongoDB database deployment target you want to test):
 
```console
cargo build
cargo run -- 'mongodb+srv://myusr:pswd@mycluster.a113z.mongodb.net'
```

 * _OPTIONAL_: Build a _production release_ version of the project's executable:
```console
cargo build --release
```
 
 * _OPTIONAL_: Run the project's _unit tests_:
```console
cargo test
```
 
 * _OPTIONAL_: Run the project's _integrations tests_ (note, requires specific MongoDB deployments to already be running, which are not documented here; also requires changing the target cluster password shown below from 'mypasswd' to the real password):
```console
TEST_PASSWD="mypasswd" cargo test -- --ignored --show-output
```
 
 * _OPTIONAL_: Run Rust's _lint_ checks against the project (to catch common mistakes and suggest where code can be improved):
```console
cargo clippy
```

 * _OPTIONAL_: Run Rust's _layout format_ checks against the project (to ensure consistent code formatting is used and highlight places where they're not):
```console
cargo fmt -- --check
```

 * _OPTIONAL_: Highlight any lines of code which have a length greater than 100 characters:
```console
cat src/main.rs | awk 'length($0) > 100'
```

## Potential Future Enhancements

* __Windows DNS Lookup Stall__. Fix stall issue on Windows, for the stages that require a DNS lookup, resulting in the tool sometimes appearing to hang for a period of time at the different affected stages.

