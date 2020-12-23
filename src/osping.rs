use std::io::ErrorKind;
use std::process::Command;


const PING_CMD: &str = "ping";
const UNIX_INTERVAL_ARG: &str = "-i 0.2";
const UNIX_COUNT_ARG: &str = "-c 3";
const WINDOWS_COUNT_ARG_KEY: &str = "-n";
const WINDOWS_COUNT_ARG_VAL: &str = "3";


// Capture type of result from issuing a ping
pub enum PingResult {
    ConnectionSuccess,
    ConnectionFailure(String),
    DNSIssue(String),
    OSCmndIssue(String),
}


// Uses the underlying OS ping executable, on the host, to perform a network ICMP ping against a
// host (DNS name or IP address). Returns a result typed to indicate success or the category of
// failure. Avoids opening an ICMP raw socket directly in Rust as this would require this Rust
// application to have elevated OS privileges (hence doesn't use an existing Rust 'ping' library).
//
pub fn ping(host: &str) -> PingResult {
    let mut cmd = &mut Command::new(PING_CMD);

    if cfg!(windows) {
        cmd = cmd.arg(WINDOWS_COUNT_ARG_KEY).arg(WINDOWS_COUNT_ARG_VAL);
    } else {
        cmd = cmd.arg(UNIX_COUNT_ARG).arg(UNIX_INTERVAL_ARG);
    }

    match cmd.arg(host).output() {
        Ok(output) => {
            if output.status.success() {
                PingResult::ConnectionSuccess
            } else if !cfg!(windows) && (output.status.code().unwrap_or(-1) == 1) {
                // Unix  (Unix's Ping uses code 1 for connection error & code 2 for other errors)
                PingResult::ConnectionFailure(format!("Host '{}' cannot be reached over a network \
                    ICMP Ping", host))
            } else {
                // Windows for all errors, Unix for non-connection related errors (code 2)
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stdout.contains("could not find host") {
                    // Windows
                    PingResult::DNSIssue(format!("Ping returned error indicating no DNS entry for \
                        '{}'. OS output received: '{}'", host, stdout))
                } else if stderr.contains("not known") {
                    // Unix
                    PingResult::DNSIssue(format!("Ping returned error indicating no DNS entry for \
                        '{}'. OS output received: '{}'", host, stderr))
                } else if stderr.contains("associated with hostname") {
                    // Unix
                    PingResult::DNSIssue(format!("Ping returned error indicating the DNS entry is \
                        not a hostname associated with an IP address. OS output received: '{}'",
                        stderr))
                } else if cfg!(windows) {
                    // Windows (Window's Ping uses stdout for errors rather than stderr)
                    PingResult::ConnectionFailure(format!("Ping returned error. OS output received \
                        - stdout: '{}' - stderr: '{}'", stdout, stderr))
                } else {
                    // Unix (Ping on Linux correctly uses stderr)
                    PingResult::ConnectionFailure(format!("Ping returned error. OS output \
                        received: '{}'", stderr))
                }
            }
        }
        Err(e) => {
            // Errors related to not being able to invoke the Ping executable on Windows / Unix
            if e.kind() == ErrorKind::NotFound {
                PingResult::OSCmndIssue("Unable to locate 'ping' executable in the local OS \
                    environment - ensure this executable is on your environment path (check your \
                    PATH environment variable)".to_string())
            } else if e.kind() == ErrorKind::PermissionDenied {
                PingResult::OSCmndIssue("Unable to run the 'ping' executable in the local OS \
                    environment due to lack of permissions - ensure the 'ping' command on your OS \
                    is assigned with executable permissions for your OS user running this \
                    tool".to_string())
            } else {
                PingResult::OSCmndIssue(format!("Unable to invoke the 'ping' executable on the \
                    underlying OS. OS output received: '{}'", e.to_string()))
            }
        }
    }
}

