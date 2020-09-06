use std::error::Error;
use std::str;
use std::io::{
    Error as IOError, ErrorKind
};
use std::time::Duration;
use std::process::exit;
use std::convert::From;
use std::net::{TcpStream, SocketAddr, IpAddr};
use tokio::runtime::Runtime;
use regex::Regex;
use trust_dns_resolver::{
     AsyncResolver, TokioAsyncResolver, TokioConnection, TokioConnectionProvider,
    error::ResolveError,
};
use clap::{App, Arg};
use mongodb::{
    Client,
    bson::{Bson, doc, Document},
    options::{ClientOptions, StreamAddress, Credential},
    error::{Error as MongoError, ErrorKind as MongoErrorKind}
};


const STAGE0: usize = 0;
const STAGE1: usize = 1;
const STAGE2: usize = 2;
const STAGE3: usize = 3;
const STAGE4: usize = 4;
const STAGE5: usize = 5;
const STAGE6: usize = 6;
const STAGE7: usize = 7;
const TOTAL_STAGES: usize = STAGE7 + 1;


struct Stage {
    index: usize,
    name: &'static str,
    desc: &'static str,
}


const STAGES: [Stage; TOTAL_STAGES] = [
    Stage { index: STAGE0, name: "<setup>", desc: "<setup>" },
    Stage { index: STAGE1, name: "URL-CHECK", 
            desc: "Confirm URL contains seed list of target server(s) or service name" },
    Stage { index: STAGE2, name: "MEMBERS-CHECK",
            desc: "Determine list of individual servers (look up DNS SRV service if defined)" },
    Stage { index: STAGE3, name: "DNS-IP-CHECK", 
            desc: "Determine the IP addresses of each individual server, via DNS" },
    Stage { index: STAGE4, name: "SOCKET-CHECK", 
            desc: "Confirm TCP socket can be established to one or more target servers" },
    Stage { index: STAGE5, name: "DRIVER-CHECK",
            desc: "Confirm driver can validate the URL (including SRV resolution if required)" },
    Stage { index: STAGE6, name: "DBPING-CHECK",
            desc: "Confirm driver can connect to deployment using 'dbping'" },
    Stage { index: STAGE7, name: "HEALTH-CHECK",
            desc: "Retrieve running deployment's member composition including which is primary" },
];


#[derive(Debug)]
enum StageState {
    NotApplicable,
    NotTested,
    Passed,
    Failed,
}


struct StageStatus {
    index: usize,
    state: StageState,
    advice: Vec<String>,
}

struct HostnameIP4AddressMap {
    hostname: String,
    port: Option<u16>,
    ipaddress: Option<IpAddr>,
}


const APP_NAME: &str = "mongo-connection-check";
const MONGO_SRV_PREFIX: &str = "mongodb+srv://";
const MONGO_SRV_LOOKUP_PREFIX: &str = "_mongodb._tcp.";
const CONNECTION_TIMEOUT_SECS: u64 = 4;
const MONGODB_DEFAULT_LISTEN_PORT: u16 = 27017;
const ERR_MSG_PREFIX: &str = " ERROR: ";
const WRN_MSG_PREFIX: &str = " WARNING: ";
const INF_MSG_PREFIX: &str = " * ";


type AsyncDnsResolver = AsyncResolver<TokioConnection, TokioConnectionProvider>;


// Main application start point processing startup args
//
fn main() {
    let args = App::new("MongoDB Connection Check")
        .version("0.8.1")
        .before_help("")
        .about("\nChecks the connectivity from your machine to a remote MongoDB deployment \n\
            If a connection can't be made, outputs advice on how to diagnose and potentially fix \n\
            The MongoDB deployment may be hosted in Atlas or may be self-managed on-prem/in-cloud")
        .arg(Arg::with_name("username")
            .short("u")
            .long("username")
            .value_name("username")
            .help("Optional username for authentication,\n(if specified, overrides any username \
                defined in the MongoDB URL)")
            .takes_value(true))
        .arg(Arg::with_name("password")
            .short("p")
            .long("password")
            .value_name("password")
            .help("Password for authentication,\n(if specified, overrides any password defined in \
                the MongoDB URL)")
            .takes_value(true))
        .arg(Arg::with_name("url")
            .value_name("url")
            .required(true)
            .help("The URL of the MongoDB deployment with a prefix of 'mongodb+srv://' or \
                'mongodb://'\n(URL should contain any required options, e.g. 'tls=true'. \
                'replicaSet=MyRS')\n(for URL format help: \
                https://docs.mongodb.com/manual/reference/connection-string/)")
            .takes_value(true))                                           
        .after_help("EXAMPLES:\n    \
             # Connect to Atlas cluster with username & password embedded in URL\n    \
             ./mongo-connection-check \"mongodb+srv://myusr:pswd@ctr.a1b2.mongodb.net/\
             ?retryWrites=true\"\n\n    \
             # Connect to Atlas cluster with username & password passed in as parameters\n    \
             ./mongo-connection-check -u mysyr -p pswd \"mongodb+srv://ctr.a1b2s.mongodb\
             .net/?retryWrites=true\"\n\n    \
             # Connect to self-managed cluster with username & password embedded in URL\n    \
             ./mongo-connection-check \"mongodb://clstr1.acme.com:27017,clstr2.acme.net:27017/\
             test?tls=true\"\n\n")
        .get_matches();
                       
    let url = args.value_of("url").expect("missing url - shouldn't happen as arg is mandatory");
    
    if url.is_empty() {
        println!("Empty URL parameter defined; terminating with error");
        exit(1);        
    }
    
    let url = args.value_of("url").expect("missing url - shouldn't happen as arg is mandatory");
    
    if url.is_empty() {
        println!("Empty URL parameter defined; terminating with error");
        exit(1);        
    }
    
    start(&url, args.value_of("username"), args.value_of("password"));
}


// Core application async bootstrap starting point
//
fn start(url: &str, username: Option<&str>, password: Option<&str>) {
    print_intro_with_stages();   
    let mut stages_status = initialise_stages_status();
    println!("Specified deployment URL:");
    println!("  '{}'", url);
    println!();        
    let mut rt = Runtime::new().unwrap();
    
    match rt.block_on(run_checks(&mut stages_status, url, username, password)) {
        Ok(_) => {
            print_summary_with_stages(&stages_status);
        }
        Err(e) => {
            println!(" UNDERLYING ERROR: {}", e.to_string());
            print_summary_with_stages(&stages_status);
            exit(1);
        }
    }
}


// Run each check serially
//
async fn run_checks(stages_status : &mut [StageStatus], url: &str, usr: Option<&str>,
                    pwd: Option<&str>) -> Result<(), Box<dyn Error>> {
    let dns_resolver =  TokioAsyncResolver::tokio_from_system_conf().await?;              
    // STAGE 1:
    let cluster_seed_list = stage1_url_check(STAGE1, stages_status, url)?;
    // STAGE 2:
    let cluster_address = stage2_members_check(STAGE2, stages_status, &dns_resolver, url, 
        &cluster_seed_list).await?;
    // STAGE 3:
    let hostname_ipaddr_mappings = stage3_dns_ip_check(STAGE3, stages_status, &dns_resolver, 
        &cluster_address).await?;
    // STAGE 4:
    stage4_ip_socket_check(STAGE4, stages_status, &hostname_ipaddr_mappings)?;
    // STAGE 5
    let client_options = stage5_driver_check(STAGE5, stages_status, url, usr, pwd).await?;
    // STAGE 6:
    let shared_tier = stage6_dbping_check(STAGE6, stages_status, url, &client_options).await?;
    // STAGE 7:
    stage7_health_check(STAGE7, stages_status, &client_options, shared_tier).await?;
    Ok(())
}


// Confirm URL contains seed list of target server(s) or service name
//
fn stage1_url_check(stage_index: usize, stages_status : &mut [StageStatus], url: &str)
                    -> Result<Vec<StreamAddress>, Box<dyn Error>> {
    print_stage_header(stage_index);
    stages_status[stage_index].state = StageState::Failed;
    
    let cluster_seed_list = match extract_cluster_seedlist(url) {
        Ok(cluster_seed_list) => cluster_seed_list,
        Err(e) => { 
            println!("{}A seed list of target server(s) or SRV service was not found in URL: '{}'",
                ERR_MSG_PREFIX, url);
            stages_status[stage_index].advice.push("Check the URL and ensure its parameters are \
                well formed and MATCH THE FORMAT specification documented at: \
                https://docs.mongodb.com/manual/reference/connection-string/".to_string());
            return Err(e);
        }
    };

    println!("{}Deployment seed list specified as: '{}'", INF_MSG_PREFIX, 
        get_displayable_addresses(&cluster_seed_list));
    
    if is_srv_url(url) {
        if cluster_seed_list.len() > 1 {
            const MSG: &str = "SRV based server name specified but more than 1 service name is \
                provided";
            stages_status[stage_index].advice.push("Fix the URL to only have one SRV name defined \
                when using a 'mongodb+srv://' prefix in the URL, or just use a 'mongodb://' prefix \
                instead, and specify one or more server 'hostname:port' combinations, each \
                separated by a comma - for more information on the URL format see: \
                https://docs.mongodb.com/manual/reference/connection-string/".to_string());
            println!("{}{}", ERR_MSG_PREFIX, MSG);
            return Err(MSG.into());        
        } else {        
            println!("{}The seed list part of the URL is valid as it defines a single SRV service\
                , not multiple", INF_MSG_PREFIX);    
            println!("{}Therefore, cluster SRV service name & port is: '{}'", INF_MSG_PREFIX,
                get_displayable_addresses(&cluster_seed_list));
        }            
    } else {
        print_address_list_members("Seed list server member", &cluster_seed_list);    
    }
       
    stages_status[stage_index].state = StageState::Passed;
    Ok(cluster_seed_list)
}


// Determine list of individual servers (looks up DNS SRV service if defined)
//
async fn stage2_members_check(stage_index: usize, stages_status : &mut [StageStatus],
                              dns_resolver: &AsyncDnsResolver, url: &str,
                              cluster_seed_list: &[StreamAddress])
                              -> Result<Vec<StreamAddress>, Box<dyn Error>> {
    print_stage_header(stage_index);
    stages_status[stage_index].state = StageState::Failed;

    let cluster_addresses =
        if is_srv_url(url) {
            match get_srv_host_addresses(dns_resolver, &cluster_seed_list).await {
                Ok(addresses) => {
                    println!("{}Successfully located a DNS SRV service record for the cluster \
                        address", INF_MSG_PREFIX);    
                    let txt_entries = get_srv_txt_options(dns_resolver, &cluster_seed_list).await?;
                    let mut has_txt_entry = false;
                    
                    for txt_entry in txt_entries {
                        println!("{}SRV service for the cluster has the following DNS TXT \
                            parameters defined which will automatically be added as connection \
                            options: '{}'", INF_MSG_PREFIX, txt_entry);
                        has_txt_entry = true;
                    }

                    if !has_txt_entry {
                        println!("{}SRV service for the cluster has DNS TXT parameters \
                            defined", INF_MSG_PREFIX);                        
                    }
                                        
                    addresses
                }
                Err(e) => {
                    println!("{}Unable to locate raw host addresses for SRV DNS service name '{}'\
                        - error message: {}", ERR_MSG_PREFIX, 
                        get_displayable_addresses(&cluster_seed_list), e.to_string());
                    const MSG: &str = "No SRV address found in URL";
                    let srv = cluster_seed_list.first().ok_or_else(|| Box::new(IOError::new(
                        ErrorKind::InvalidInput, MSG)))?;                              
                    stages_status[stage_index].advice.push(format!("From this machine launch a \
                        terminal and use the nslookup tool to query DNS for the SRV service which \
                        is supposed to return the list of actual member server hostnames and ports \
                        (if it does not, then you have a DNS problem): \
                        'nslookup -q=SRV _mongodb._tcp.{}'", srv.hostname));
                    return Err(e);
                }
            }
        } else { 
            println!("{}The seed list in the URL is based on raw host addresses rather than a \
                service name, so no need to perform a DNS SRV lookup", INF_MSG_PREFIX);    
            cluster_seed_list.to_owned()
        };

    
    let phrasing = if is_srv_url(url) {"decomposed to"} else {"verified as"};
    println!("{}Deployment seed list now {}: '{}'", INF_MSG_PREFIX, phrasing,
        get_displayable_addresses(&cluster_addresses));
    print_address_list_members("Deployment individual raw server address identified",
        &cluster_addresses);
    stages_status[stage_index].state = StageState::Passed;
    Ok(cluster_addresses)
}


// Determine the IP addresses of each individual server, via DNS
//
async fn stage3_dns_ip_check(stage_index: usize, stages_status : &mut [StageStatus],
                       dns_resolver: &AsyncDnsResolver, cluster_address: &[StreamAddress])
                       -> Result<Vec::<HostnameIP4AddressMap>, Box<dyn Error>> {
    print_stage_header(stage_index);
    stages_status[stage_index].state = StageState::Failed;
    const MSG: &str = "Unable to find any IP address mapping in DNS for any of the members of the \
        deployment";
    const ADVC: &str = "From this machine launch a terminal and use the nslookup tool to query DNS \
        for the IP address of the server hostname  (if nothing is returned, then you have a DNS \
        problem):  'nslookup";
    let hostname_ipaddress_mappings_res = get_ipv4_addresses(dns_resolver, &cluster_address).await;
    
    let hostname_ipaddress_mappings = match hostname_ipaddress_mappings_res {
        Ok(hostname_ipaddress_mappings) => {    
            let mut found_at_least_one_ip_address = false;
        
            for hostname_ipaddress_mapping in hostname_ipaddress_mappings.iter() {        
                match hostname_ipaddress_mapping.ipaddress {
                    Some(ipaddress) => {
                        found_at_least_one_ip_address = true;
                        println!("{}Server hostname '{}' resolved to IP address: '{}'",
                        INF_MSG_PREFIX, hostname_ipaddress_mapping.hostname, ipaddress);
                    }
                    None => {
                        println!("{}No IPv4 IP address found in DNS for hostname: '{}'",
                            WRN_MSG_PREFIX, hostname_ipaddress_mapping.hostname);
                        stages_status[stage_index].advice.push(format!("{} {}'", ADVC,
                            hostname_ipaddress_mapping.hostname));
                    }
                };
            }
            
            if !found_at_least_one_ip_address {
                println!("{}{} '{}'", ERR_MSG_PREFIX, MSG,
                    get_displayable_addresses(cluster_address));
                return Err(MSG.into());        
            }
            
            hostname_ipaddress_mappings
        }
        Err(e) => {
                println!("{}{} '{}' - error message: {}", ERR_MSG_PREFIX, MSG, 
                    get_displayable_addresses(cluster_address), e.to_string());
                let first_addr = cluster_address.first()
                    .ok_or_else(|| Box::new(IOError::new(ErrorKind::InvalidInput,
                    "Got no hostnames to lookup")))?;
                stages_status[stage_index].advice.push(format!("{} {}'", ADVC, 
                    first_addr.hostname));
                return Err(Box::new(e));        
        }
    };
                    
    stages_status[stage_index].state = StageState::Passed;
    Ok(hostname_ipaddress_mappings)
}


// Confirm TCP socket can be established to one or more target servers
//
fn stage4_ip_socket_check(stage_index: usize, stages_status : &mut [StageStatus],
                          hostname_ipaddr_map: &[HostnameIP4AddressMap])
                          -> Result<(), Box<dyn Error>> {
    print_stage_header(stage_index);
    stages_status[stage_index].state = StageState::Failed;
    let mut connect_success_count = 0;
    let mut resume_os_advice_count_given = false;
    
    for hostnm_ipaddr_map in hostname_ipaddr_map {      
        let port = &hostnm_ipaddr_map.port.unwrap_or(MONGODB_DEFAULT_LISTEN_PORT);
        let ipaddress = match hostnm_ipaddr_map.ipaddress {
            Some(value) => value,
            None => {
                println!("{}Skipping attempt to open a TCP socket connection to '{}:{}' because \
                    its IP address was not previously resolved in DNS (see warning in previous \
                    stage)", INF_MSG_PREFIX, &hostnm_ipaddr_map.hostname, port);
                continue;
            }
        };

        // NOTE: For shared tiers can still open a socket even if whitelist in place            
        connect_success_count += match try_open_client_tcp_connection(&hostnm_ipaddr_map.ipaddress,
                                                                      &hostnm_ipaddr_map.port) {
            Ok(_) => {                                               
                println!("{}TCP socket connection successfully opened to server '{}:{}' \
                    (IP address: '{}')", INF_MSG_PREFIX, &hostnm_ipaddr_map.hostname, port,
                   ipaddress);
                1
            }
            Err(e) => {
                let err_msg = e.to_string();
                println!("{}Unable to open TCP socket connection to IP Address: '{}' (for server \
                    '{}:{}') - error message: {}", WRN_MSG_PREFIX, ipaddress, 
                    &hostnm_ipaddr_map.hostname, port, err_msg);
                         
                if !resume_os_advice_count_given && err_msg.contains("os error 111") {
                    stages_status[stage_index].advice.push("The type of TCP connection error \
                        received indicates that the host machines are running and generally \
                        accessible but the hosted MongoDB servers are not yet fully up and \
                        running and so CANNOT ACCEPT the socket connection. CHECK THE STATE of \
                        the MongoDB deployment servers, in case there is a problem there. If \
                        deployed to Atlas, this situation can happen if the cluster had been \
                        paused and is now resuming (if this is the case, check the Atlas console \
                        to see when the cluster is fully resumed and then just try this \
                        connection check again)".to_string());
                    resume_os_advice_count_given = true;
                }
                
                stages_status[stage_index].advice.push(format!("From this machine launch a \
                    terminal and use the netcat tool to see if a socket can be successfully opened \
                    to the server:port:  'nc -zv -w 5 {} {}'", &hostnm_ipaddr_map.hostname, port));
                0
            }
        }
    }
        
    if connect_success_count <= 0 {
        const MSG: &str = "Unable to open a TCP socket connection to any of the server addresses \
            derived from the URL's seed list";    
        println!("{}{}", ERR_MSG_PREFIX, MSG);
        capture_no_connection_advice(&mut stages_status[stage_index]);
        return Err(MSG.into());
    }

    stages_status[stage_index].state = StageState::Passed;
    Ok(())
}


// Confirm driver can validate the URL (including SRV resolution if required
//
async fn stage5_driver_check(stage_index: usize, stages_status : &mut [StageStatus], url: &str,
                       usr: Option<&str>, pwd: Option<&str>)
                       -> Result<ClientOptions, MongoError> {
    print_stage_header(stage_index);
    stages_status[stage_index].state = StageState::Failed;

    let client_options = match get_mongo_client_options(url, usr, pwd).await {
        Ok(client_options) => client_options,
        Err(e) => {
            println!("{}The driver found an issue in the specified URL '{}' when trying to parse \
                and process it - error message: {}", ERR_MSG_PREFIX, url, e.to_string());
            stages_status[stage_index].advice.push("Check the URL and ensure its parameters are \
                well formed and MATCH THE FORMAT specification documented at: \
                https://docs.mongodb.com/manual/reference/connection-string/".to_string());
            return Err(e);
        }
    };
    
    for host in client_options.hosts.iter() {
        println!("{}From the specified URL, the driver resolved the following member server: \
            '{}:{}'", INF_MSG_PREFIX, host.hostname,
            host.port.unwrap_or(MONGODB_DEFAULT_LISTEN_PORT));
    }

    stages_status[stage_index].state = StageState::Passed;
    Ok(client_options)
}


// Confirm driver can connect to deployment using 'dbping' (warn if errors occur)
//
async fn stage6_dbping_check(stage_index: usize, stages_status : &mut [StageStatus],
                             url: &str, client_options: &ClientOptions)
                             -> Result<bool, Box<dyn Error>> {
    print_stage_header(stage_index);
    stages_status[stage_index].state = StageState::Failed;
    let mut shared_tier = false;

    match get_dbping_response(client_options).await {
        Ok(dbping_response) => {
            let dbping_ok = match dbping_response.get("ok") {        
                Some(dbping_val) => {
                    match dbping_val {
                        Bson::Int32(val) => {
                            shared_tier = true;
                            *val == 1
                        }
                        Bson::Double(val) => {
                            // MORE PRECISE FOR FLOATING POINTS THAN: *val == 1.0
                            (*val - 1.0).abs() < f64::EPSILON
                        }                     
                        Bson::String(val) => val.eq("1") || val.eq("1.0"),
                        Bson::Boolean(val) => *val,
                        Bson::Int64(val) => *val == 1,
                        _ => false,
                    }
                }
                None => false,
            };
                                        
            if dbping_ok {
                println!("{}The driver successfully connected to the deployment, with the \
                    'dbping' command returning OK", INF_MSG_PREFIX);
            } else {
                const MSG: &str = "The driver was able to establish an initial connection to the \
                    deployment but the 'dbping' command failed";                
                println!("{}{}", ERR_MSG_PREFIX, MSG);
                stages_status[stage_index].advice.push("Using the Mongo Shell, connect to the \
                    MongoDB deployment and run the 'db.runCommand({ping: 1})' command to check the \
                    health of the deployment and to see if any issues are reported".to_string());
                return Err(MSG.into());                        
            }
        }
        Err(e) => {
            // Downcast err from std::boxed::Box<dyn std::error::Error> to: &mongodb::error::Error
            match e.downcast_ref::<MongoError>() {
                Some(err) => alert_on_db_error_type(&mut stages_status[stage_index], url, &err),
                None =>  println!("{}The driver was unable to establish a valid network connection \
                            to the MongoDB deployment - error message: {}", ERR_MSG_PREFIX, 
                            e.to_string()),
            }
            
            return Err(e);          
        }
    };

    stages_status[stage_index].state = StageState::Passed;
    Ok(shared_tier)
}


// Retrieve the running deployment's member composition & which is the primary
//
async fn stage7_health_check(stage_index: usize, stages_status : &mut [StageStatus],
                       client_options: &ClientOptions, shared_tier: bool) 
                       -> Result<(), Box<dyn Error>> {
    print_stage_header(stage_index);
    stages_status[stage_index].state = StageState::Failed;
    let mut is_identified = false;
    const ADVC: &str = "Using the Mongo Shell, connect to the MongoDB deployment and run the \
        'db.isMaster()' command to check the health of the deployment and to see if any issues are \
        reported";

    match get_dbismaster_response(client_options).await {
        Ok(doc) => {
            if let Ok(msg) = doc.get_str("msg") {
                if msg == "isdbgrid" {
                    println!("{}Issued command 'ismaster' indicates that a mongos router has been \
                        connected to and the deployment is Sharded", INF_MSG_PREFIX);
                    is_identified = true;
                }            
            } 
            
            if !is_identified && shared_tier {            
                println!("{}MongoDB response indicates that the deployment is an Atlas shared tier \
                    (M0, M2 or M5) based on a shared replica set", INF_MSG_PREFIX);
                is_identified = true;
            }            

            if !is_identified {            
                if let Ok(primary) = doc.get_str("primary") {
                    if let Ok(hosts) = doc.get_array("hosts") {
                        println!("{}Issued command 'ismaster' indicates that a replica set \
                            deployment has been connected to", INF_MSG_PREFIX);

                        for address_bson in hosts {
                            let host = address_bson.to_string().trim_matches('"').to_owned();
                            let member_type = if host.eq(&primary) { "PRIMARY" } 
                                              else { "SECONDARY" };
                            println!("{}Issued command 'ismaster' lists one of the replica set \
                                members as: '{}' ({})", INF_MSG_PREFIX, host, member_type);
                            is_identified = true;
                        }
                    } 
                }                 
            }
            
            if !is_identified {            
                if let Ok(ismaster) = doc.get_bool("ismaster") {
                    if ismaster {
                        println!("{}Issued command 'ismaster' indicates that a standalone mongod \
                            server deployment has been connected to ", INF_MSG_PREFIX);
                        is_identified = true;
                    }            
                } 
            }
        }
        Err(e) => {
            println!("{}The driver received an error when trying to issue the 'ismaster' command - \
                error message: {}", ERR_MSG_PREFIX, e.to_string());
            stages_status[stage_index].advice.push(ADVC.to_string());                     
            return Err(e);
        }
    };
        
    if !is_identified {
        const MSG: &str = "The driver returned an empty list of server members in response to \
            the 'ismaster' command and the deployment cannot correctly be identified as a \
            standalone server, a replica set or a sharded deployment";                
        println!("{}{}", ERR_MSG_PREFIX, MSG);
        stages_status[stage_index].advice.push(ADVC.to_string());                     
        return Err(MSG.into());                        
    }
    
    stages_status[stage_index].state = StageState::Passed;
    Ok(())
}


// Return true if start of url indicate SRV service name specified
//
fn is_srv_url(url: &str) -> bool {
    url.starts_with(MONGO_SRV_PREFIX)
}


// Create a string representation of all the addreses, comma separated
//
fn get_displayable_addresses(addresses: &[StreamAddress]) -> String {
    let address_str_list: Vec<String> = addresses.iter().map(|addr| get_displayable_address(addr))
        .collect(); 
    
    address_str_list.join(",")
}


// Concatenate hostname and port into string separated by ':'
// 
fn get_displayable_address(address: &StreamAddress) -> String {
    format!("{}:{}", address.hostname, address.port.unwrap_or(MONGODB_DEFAULT_LISTEN_PORT))
}


// Parse the URL extracting the seed list part (one or more server[:port] elements)
// 
fn extract_cluster_seedlist(url: &str) -> Result<Vec<StreamAddress>, Box<dyn Error>> {
    let regex = Regex::new(r"^mongodb(?:\+srv)??://(?:.*@)?(?P<address>[^/&\?]+)")?;
    let err_msg = format!("Unable to find a seed list in the provided MongoDB URL: '{}'", url);

    let seedlist_option = match regex.captures(url) {
        Some(captured) => captured.name("address").map(|m| m.as_str()),        
        None => return Err(err_msg.into())
    };

    let seedlist = match seedlist_option {
        Some(text) => text,
        None => return Err(err_msg.into())
    };

    let cluster_seed_list: Result<Vec<_>, _> = seedlist.split(',')
        .map(|res| StreamAddress::parse(res)).collect();
    Ok(cluster_seed_list?)  // Need to unwrap and rewrap so that the error is boxed
}


// Perform a DNS SRV lookup for a service name returning hostnames this maps to
// 
async fn get_srv_host_addresses(dns_resolver: &AsyncDnsResolver, 
                                cluster_seed_list: &[StreamAddress])
                                -> Result<Vec<StreamAddress>, Box<dyn Error>> {
    const MSG: &str = "No address found in URL ready for SRV DNS lookup";
    let address = cluster_seed_list.first().ok_or_else(|| Box::new(IOError::new(
        ErrorKind::InvalidInput, MSG)))?;    
    let srv_hostname_query = format!("{}{}.", MONGO_SRV_LOOKUP_PREFIX, address.hostname);
    let lookup_response = dns_resolver.srv_lookup(srv_hostname_query).await?;

    let srv_addresses: Vec<_> = lookup_response.iter()
        .map(|record| {
            let hostname = record.target().to_utf8().trim_end_matches('.').to_owned();
            let port = Some(record.port());
            StreamAddress {hostname, port}
        }).collect();
    
    Ok(srv_addresses)    
}


// Perform a DNS TXT lookup for a service name returning any defined connection options
// 
async fn get_srv_txt_options(dns_resolver: &AsyncDnsResolver, cluster_seed_list: &[StreamAddress])
                             -> Result<Vec<String>, Box<dyn Error>> {
    const MSG: &str = "No address found in URL ready for TXT DNS lookup";
    let address = cluster_seed_list.first().ok_or_else(|| Box::new(IOError::new(
        ErrorKind::InvalidInput, MSG)))?;    
    let txt_hostname_query = format!("{}.", address.hostname);                
    let lookup_response = dns_resolver.txt_lookup(txt_hostname_query).await?;    
    let mut string_list = vec![];

    for txt_rr in lookup_response.iter() {
        for txt in txt_rr.iter() {
            string_list.push(str::from_utf8(&*txt)?.to_owned());
        }
    }        

    Ok(string_list)
}


// Perform a DNS lookup of the IP address for a given hostname
//
async fn get_ipv4_addresses(dns_resolver: &AsyncDnsResolver, cluster_addresses: &[StreamAddress])
                            -> Result<Vec<HostnameIP4AddressMap>, ResolveError> {
    let mut dns_mappings = Vec::<HostnameIP4AddressMap>::new();
    let mut first_err = None;

    for server_address in cluster_addresses {
        let mut mapping = HostnameIP4AddressMap { hostname: server_address.hostname.to_string(),
            ipaddress: None, port: server_address.port };
        let hostname_ip_query = format!("{}.", server_address.hostname);                
        let lookup_response_wrp = dns_resolver.lookup_ip(hostname_ip_query).await;
        
        match lookup_response_wrp {
            Ok(lookup_response) => {                     
                for ipaddress in lookup_response.iter() {
                    if ipaddress.is_ipv4() {
                        mapping.ipaddress = Some(ipaddress);
                        break;
                    }
                }                           
            }
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
        
        dns_mappings.push(mapping);       
    } 
    
    if dns_mappings.is_empty() {
        match first_err {
            Some(e) => Err(e),
            None => Err("DNS IP Lookup Unknown Failure".into()),             
        }
    } else {        
        Ok(dns_mappings)
    }
}


// Attempt to open TCP connection to deployment returning OK if successful or throwing error it not
//
fn try_open_client_tcp_connection(ipaddress_option: &Option<IpAddr>, port_option: &Option<u16>)
                                  -> Result<(), Box<dyn Error>> {
    const MSG: &str = "Issue testing socket connection because received IP address is undefined";
    let ipaddress = ipaddress_option.ok_or_else(|| Box::new(IOError::new(ErrorKind::InvalidInput,
        MSG)))?;
    let socket_addr = SocketAddr::new(ipaddress, port_option.unwrap_or( 
        MONGODB_DEFAULT_LISTEN_PORT));
    TcpStream::connect_timeout(&socket_addr, Duration::new(CONNECTION_TIMEOUT_SECS, 0))?;
    Ok(())
}


// Invoke MongoDB Rust Driver client_options parser adding credentials if specified
// 
async fn get_mongo_client_options(url: &str, usr: Option<&str>, pwd: Option<&str>) 
                            -> Result<ClientOptions, MongoError> {
    let mut client_options = ClientOptions::parse(url).await?;
    client_options.app_name = Some(APP_NAME.to_string());       
    client_options.server_selection_timeout = Some(Duration::new(CONNECTION_TIMEOUT_SECS, 0));
    
    let mut credentials_modified = false;
    
    let mut cred = match client_options.credential {
        Some(ref credentials) => credentials.clone(),
        None => Credential::builder().build()
    };
    
    if let Some(username) = usr {
        cred.username = Some(username.to_string());
        credentials_modified = true;
    }

    if let Some(password) = pwd {
        cred.password = Some(password.to_string());
        credentials_modified = true;
    }
    
    if credentials_modified {
        client_options.credential = Some(cred);        
    }
    
    Ok(client_options)
}


// Issue MongoDB Driver dbping command to deployment and return the command's result document
//
async fn get_dbping_response(client_options : &ClientOptions) -> Result<Document, Box<dyn Error>> {
    let client = Client::with_options(client_options.to_owned())?;
    let database = client.database("test");
    Ok(database.run_command(doc! {"ping": 1}, None).await?)      
}


// Issue MongoDB Driver ismaster command to deployment and return the command's result document
// 
async fn get_dbismaster_response(client_options : &ClientOptions)
                                 -> Result<Document, Box<dyn Error>> {
    let client = Client::with_options(client_options.to_owned())?;
    let database = client.database("test");
    Ok(database.run_command(doc! {"isMaster": 1}, None).await?)
}


// Collect together advice when a connection cannot be made to any server in the deployment
//
fn capture_no_connection_advice(stg : &mut StageStatus) {
    stg.advice.push("If using Atlas, via the Atlas console, in the 'Network Access' section, for \
        the 'IP Access List' tab ensure this machine is listed in the access list, and if not, add \
        it ('Add Current IP Address')".to_string());
    stg.advice.push("If using Atlas, via the Atlas console, check the cluster is NOT PAUSED and \
        resume it if is paused".to_string());
    stg.advice.push("If not using Atlas to host the MongoDB deployment, check the firewall rules \
        for the network hosting the deployment to ensure it permits MongoDB TCP connections on \
        the configured ports - check NOT BLOCKED".to_string());
    stg.advice.push("Check any local firewalls on this machine and in your local network, to \
        ensure that MongoDB TCP network connections to outside your network are NOT BLOCKED"
        .to_string());
}

// Print specific error message and advice depending on the kind of MongoDB driver error received.
//
// Notes:
// * The kind's type is: `std::sync::Arc<mongodb::error::ErrorKind> (as has now been dereferenced)
// * The kind's type with a * dereference is: `mongodb::error::ErrorKind`
// * When kind is dereferenced it needs to be & borrowed again to allow match to access the struct
fn alert_on_db_error_type(stg : &mut StageStatus, url: &str, err: &MongoError) {
    let kind = &err.kind;
    
    match &*kind.to_owned() {
        MongoErrorKind::ServerSelectionError{ message, .. } => {              
            if message.contains("os error 104") {
                println!("{}The driver was unable to establish a valid connection to the \
                    deployment, but given that a TCP connection was achieved in an earlier stage, \
                    this may indicate that the deployment is an Atlas shared tier (M0, M2 or M5) \
                    based on a shared replica set, but currently with no matching IP Access List \
                    entry defined to enable access from this machine.   Detail: {}",
                    ERR_MSG_PREFIX, message);                     
            } else if message.contains("unexpected end of file") {
                println!("{}The driver was unable to establish a valid connection to the \
                    deployment, but given that a TCP connection was achieved in an earlier stage, \
                    this may indicate that the deployment is configured with TLS/SSL but the URL \
                    used does not reflect this, and/or is an Atlas shared tier (M0, M2 or M5) \
                    based on a shared replica set, but currently with no matching IP Access List \
                    entry defined to enable access from this machine.   Detail: {}", 
                    ERR_MSG_PREFIX, message);
            } else {
                println!("{}The driver was unable establish a valid connection to any server in \
                    the MongoDB deployment, so cannot perform a server selection.   Detail: {}", 
                    ERR_MSG_PREFIX, message);
            }   
                                   
            capture_older_atlas_versions_advice_if_affected(stg, message);

            if message.contains("os error 104") || !is_srv_url(url) {
                capture_some_optional_advice_if_affected(stg, url, message);
                capture_no_connection_advice(stg);                
            } else {
                capture_no_connection_advice(stg);
                capture_some_optional_advice_if_affected(stg, url, message);
            }
        }
        MongoErrorKind::AuthenticationError{ message, .. } => {
            println!("{}The driver was able to establish a TCP connection to at least one server \
                in the MongoDB deployment, but failed to authenticate using the provided username/\
                password/authSource.   Detail: {}", ERR_MSG_PREFIX, message);
            stg.advice.push("Check the USERNAME you provided in the MongoDB URL or as a parameter, \
                if you specified one, to ensure it matches a configured database user in the \
                target MongoDB deployment".to_string());
            stg.advice.push("Check the PASSWORD you provided in the MongoDB URL or as a parameter, \
                if you specified one, to ensure it is correct for the configured database user in \
                the target MongoDB deployment".to_string());
            stg.advice.push("Check an 'authSource' URL option has been provided (e.g. \
               '&authSource=admin') and its value is correct for the configured target MongoDB \
               deployment".to_string());
        }
        MongoErrorKind::ArgumentError{ message, .. } => {
            println!("{}The driver found problems in the URL string specified and therefore did \
                not attempt to test TCP connectivity.   Detail: {}", ERR_MSG_PREFIX, message);
            stg.advice.push("Check the URL and ensure its parameters are well formed and MATCH THE \
                FORMAT specification documented at: 
                https://docs.mongodb.com/manual/reference/connection-string/".to_string());
        }
        _ => {
            let err_msg = err.to_string();      
            println!("{}The driver was unable to establish a valid connection to the MongoDB \
                deployment.   Detail:: {}", ERR_MSG_PREFIX, err_msg);
            capture_older_atlas_versions_advice_if_affected(stg, &err_msg);                      
            stg.advice.push("Check the FIREWALL RULES of both your client network and the network \
                hosting the MongoDB deployment, for rules that prevent MongoDB TCP connections to \
                the MongoDB deployment member ports".to_string());
        }
    };
}


// Collect together advice depending on context (e.g what is in the url, or error code received)
//
fn capture_some_optional_advice_if_affected(stg : &mut StageStatus, url: &str, errmsg: &str) {
    if errmsg.contains("os error 104") {
        stg.advice.push("If the MongoDB deployment is an Atlas M0/M2/M5 tier cluster then via the \
            Atlas console, in the 'Network Access' section, for the 'IP Access List' tab select to \
            'ADD CURRENT IP ADDRESS' which should be the address of this host machine".to_string());
    } else if errmsg.contains("unexpected end of file") && 
              !url.contains("tls=true") &&
              !url.contains("ssl=true") {
        stg.advice.push("The type of error received indicates that the MongoDB deployment may be \
            configured with TLS (aka SSL) which it should be, for security reasons. However, the \
            MongoDB URL you provided does not seem to indicate that the driver should communicate \
            via TLS. ADD THE CLIENT OPTION 'tls=true' to the MongoDB URL you specify, as per the \
            following format specification, and try again: \
            https://docs.mongodb.com/manual/reference/connection-string/".to_string());
    }      
                           
    if !url.contains('?') {
        stg.advice.push("Ensure you have used a ? (QUESTION MARK) in the MongoDB URL you specified\
            , before any connection string options you defined, and also check each of these \
            options is VALID as per the format specification documented at: \
            https://docs.mongodb.com/manual/reference/connection-string/".to_string());
    }    
}


// Advise if deployment suspected of running older versions of MongoDB on Atlas with TLS/SSL issues
//
fn capture_older_atlas_versions_advice_if_affected(stg : &mut StageStatus, errmsg: &str) {
    if errmsg.contains("tls handshake eof") {
        stg.advice.push("UNSUPPORTED by this utility - it appears that you are targetting a \
            MongoDB cluster deployed in Atlas, which is running a version of MongoDB lower than \
            4.2 (e.g. version 3.6 or 4.0). Unfortunately, due to older TLS/SSL libraries used on \
            the hosts' OS for those MongoDB versions, in Atlas, the Rust driver, used by this \
            connection test utility, will be unable to connect to the database".to_string());
    }
}


// Print out the address of each server in a list
//
fn print_address_list_members(prefix: &str, addresses: &[StreamAddress]) {
    let mut i = 1;
    
    for address in addresses.iter() {
        println!("{}{} #{}: '{}'", INF_MSG_PREFIX, prefix, i, get_displayable_address(address));
        i += 1;
    }
}


// Print out the opening application output describing the stages that will be run
// 
fn print_intro_with_stages() {
    println!();
    println!();    
    println!("======= STARTED: {} =======", APP_NAME);
    println!();
    println!("CHECKS TO BE ATTEMPTED: ");

    for stage in &STAGES[1..] {
        println!(" {}. {}:  \t {}", stage.index, stage.name, stage.desc);
    }

    println!();
    println!("-----------------------------------------------");
    println!();
}


// Print out the final summary of the result of each check + any resulting advice
// 
fn print_summary_with_stages(stages_status : &[StageStatus]) {
    println!();
    println!();
    println!("-----------------------------------------------");
    println!();
    println!("FINAL STATUS OF CHECKS: ");
    
    for stage in &stages_status[1..] {
        println!(" {}. {}:  \t {:#?}", stage.index, STAGES[stage.index].name, stage.state);
    }

    let mut advice_header_shown = false;
    
    for stage in &stages_status[1..] {
        for advice in &stage.advice {
            if !advice_header_shown {
                println!();
                println!("RESULTING ADVICE: ");
            }

            println!(" - {}  ({})", advice, STAGES[stage.index].name);            
            advice_header_shown = true;
        }
    }
    println!();
    println!("======= ENDED: {} ========", APP_NAME);
    println!();    
    println!();
}


// Print out the number and name of the stage about to be run
//
fn print_stage_header(stage_index: usize) {
    println!();
    println!("----- STAGE {:#?} ({}) -----", stage_index, STAGES[stage_index].name);   
    println!();
}


// Initialise the stages check tracking structure to all be 'NotTested', initially
// 
fn initialise_stages_status() -> [StageStatus; TOTAL_STAGES] {
    [
        StageStatus { index: STAGE0, state: StageState::NotApplicable, advice: vec![] },
        StageStatus { index: STAGE1, state: StageState::NotTested, advice: vec![] },
        StageStatus { index: STAGE2, state: StageState::NotTested, advice: vec![] },
        StageStatus { index: STAGE3, state: StageState::NotTested, advice: vec![] },
        StageStatus { index: STAGE4, state: StageState::NotTested, advice: vec![] },
        StageStatus { index: STAGE5, state: StageState::NotTested, advice: vec![] },
        StageStatus { index: STAGE6, state: StageState::NotTested, advice: vec![] },
        StageStatus { index: STAGE7, state: StageState::NotTested, advice: vec![] },
    ]
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_srv() {
        assert!(!is_srv_url("mongodb://abc123.mongodb.net/test"));
        assert!(is_srv_url("mongodb+srv://seb360.o5qhl.gcp.mongodb.net/test"));
        assert!(!is_srv_url("mongodb+srv:seb360.o5qhl.gcp.mongodb.net/test"));
    }


    #[test]
    fn test_addresses_display() {
        let mut address_list = vec![];
        address_list.push(StreamAddress::parse("abc123.mongodb.com:27017").unwrap());
        address_list.push(StreamAddress::parse("xyz789.mongodb.com:27017").unwrap());
        assert_eq!(get_displayable_addresses(&address_list), 
            "abc123.mongodb.com:27017,xyz789.mongodb.com:27017");
        let mut address_list = vec![];
        address_list.push(StreamAddress::parse("abc123.mongodb.com:27017").unwrap());
        address_list.push(StreamAddress::parse("pqr456.mongodb.com").unwrap());
        address_list.push(StreamAddress::parse("xyz789.mongodb.com:27017").unwrap());
        assert_eq!(get_displayable_addresses(&address_list),
            "abc123.mongodb.com:27017,pqr456.mongodb.com:27017,xyz789.mongodb.com:27017");
        let mut address_list = vec![];
        address_list.push(StreamAddress::parse("localhost").unwrap());
        assert_eq!(get_displayable_addresses(&address_list), "localhost:27017");
    }


    #[test]
    fn test_extract_cluster_seedlist() {
        assert_extract_cluster_seedlist("mongodb:abc123.mongodb.net:27017/test", &[], &[]);
        assert_extract_cluster_seedlist("mongodb//abc123.mongodb.net:27017/test", &[], &[]);
        assert_extract_cluster_seedlist("mongodb://abc123.mongodb.net:27017/test",
            &["abc123.mongodb.net"], &[27017]);
        assert_extract_cluster_seedlist("mongodb://abc123.mongodb.net:27017/",
            &["abc123.mongodb.net"], &[27017]);
        assert_extract_cluster_seedlist("mongodb://abc123.mongodb.net:27017",
            &["abc123.mongodb.net"], &[27017]);
        assert_extract_cluster_seedlist("mongodb://abc123.mongodb.net/test",
            &["abc123.mongodb.net"], &[]);
        assert_extract_cluster_seedlist("mongodb://abc123.mongodb.net/",
            &["abc123.mongodb.net"], &[]);
        assert_extract_cluster_seedlist("mongodb://abc123.mongodb.net",
            &["abc123.mongodb.net"], &[]);
        assert_extract_cluster_seedlist("mongodb+srv://seb360.o5qhl.gcp.mongodb.net:27017/test",
            &["seb360.o5qhl.gcp.mongodb.net"], &[27017]);
        assert_extract_cluster_seedlist("mongodb+srv://seb360.o5qhl.gcp.mongodb.net:27017/",
            &["seb360.o5qhl.gcp.mongodb.net"], &[27017]);
        assert_extract_cluster_seedlist("mongodb+srv://seb360.o5qhl.gcp.mongodb.net:27017",
            &["seb360.o5qhl.gcp.mongodb.net"], &[27017]);
        assert_extract_cluster_seedlist("mongodb+srv://seb360.o5qhl.gcp.mongodb.net/test",
            &["seb360.o5qhl.gcp.mongodb.net"], &[]);
        assert_extract_cluster_seedlist("mongodb+srv://seb360.o5qhl.gcp.mongodb.net/",
            &["seb360.o5qhl.gcp.mongodb.net"], &[]);
        assert_extract_cluster_seedlist("mongodb+srv://seb360.o5qhl.gcp.mongodb.net",
            &["seb360.o5qhl.gcp.mongodb.net"], &[]);
        assert_extract_cluster_seedlist("mongodb://mongodb1.example.com:27317,mongodb2.example.com\
            :27017/?replicaSet=mySet&authSource=authDB",
            &["mongodb1.example.com", "mongodb2.example.com"], &[27317, 27017]);
        assert_extract_cluster_seedlist("mongodb://mongodb1.example.com:27317,mongodb2.example.com\
            :27017/", &["mongodb1.example.com", "mongodb2.example.com"], &[27317, 27017]);
        assert_extract_cluster_seedlist("mongodb://mongodb1.example.com:27317,mongodb2.example.com\
            :27017", &["mongodb1.example.com", "mongodb2.example.com"], &[27317, 27017]);
        assert_extract_cluster_seedlist("mongodb://mongodb0.example.com:27017,mongodb1.example.com\
            :27017,mongodb2.example.com:27017/?replicaSet=myRepl&\
            authSource=admin", &["mongodb0.example.com", "mongodb1.example.com", 
            "mongodb2.example.com"], &[27017, 27017, 27017]);
        assert_extract_cluster_seedlist("mongodb://mongodb0.example.com:27017,mongodb1.example.com\
            :27017,mongodb2.example.com:27017?replicaSet=myRepl&\
            authSource=admin", &["mongodb0.example.com", "mongodb1.example.com", 
            "mongodb2.example.com"], &[27017, 27017, 27017]);
        assert_extract_cluster_seedlist("mongodb://mongodb0.example.com:27017,mongodb1.example.com\
            :27017,mongodb2.example.com:27017/&authSource=admin",
            &["mongodb0.example.com", "mongodb1.example.com", "mongodb2.example.com"], 
            &[27017, 27017, 27017]);
        assert_extract_cluster_seedlist("mongodb://mongodb0.example.com:27017,mongodb1.example.com\
            :27017,mongodb2.example.com:27017&authSource=admin",
            &["mongodb0.example.com", "mongodb1.example.com", "mongodb2.example.com"], 
            &[27017, 27017, 27017]);
        assert_extract_cluster_seedlist("mongodb://myuser@mongodb0.example.com:27017,\
            mongodb1.example.com:27017,mongodb2.example.com:27017/?\
            replicaSet=myRepl&authSource=admin", &["mongodb0.example.com", "mongodb1.example.com",
            "mongodb2.example.com"], &[27017, 27017, 27017]);
        assert_extract_cluster_seedlist("mongodb://myuser:mypassword@mongodb0.example.com:27017,\
            mongodb1.example.com:27017,mongodb2.example.com:27017/?\
            replicaSet=myRepl&authSource=admin", &["mongodb0.example.com", "mongodb1.example.com",
            "mongodb2.example.com"], &[27017, 27017, 27017]);
        assert_extract_cluster_seedlist("mongodb://myuser:mypassword@mongodb0.example.com,\
            mongodb1.example.com,mongodb2.example.com/?replicaSet=\
            myRepl&authSource=admin", &["mongodb0.example.com", "mongodb1.example.com", 
            "mongodb2.example.com"], &[]);
        assert_extract_cluster_seedlist("mongodb+srv://main_user:Password1@/test?retryWrites=true\
            &w=majority", &[], &[]);
    } 


    fn assert_extract_cluster_seedlist(url: &str, hosts: &[&str], ports: &[u16]) {
        match extract_cluster_seedlist(url) {
            Ok(address_list) => {
                assert_eq!(address_list.len(), hosts.len());
                let mut pos = 0;
                
                for address in address_list {                           
                    assert_eq!(address.hostname, hosts[pos]);    
                    
                    if let Some(port) = address.port {
                        assert_eq!(port, ports[pos]);      
                    }
                                        
                    pos += 1;
                }
            }
            Err(e) => {            
                // True if there was no seedlist in the URL which caused the error
                assert!((hosts.len() <= 0), e.to_string()) 
            }
        };
    }
}

