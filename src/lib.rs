use base64::Engine;
use colored::Colorize;
use tokio::{fs, io::{AsyncReadExt, AsyncWriteExt, BufReader}, process::Command, time::timeout};
use std::{process::Stdio, time::Duration};

pub mod mediawiki;

/// Fetch the currently available VPN configurations from vpngate.net, in OpenVPN format.
/// Returns a tuple:
/// 0 is vector of strings, each string containing the contents of the OpenVPN configuration file needed to connect to that VPN server.
/// 1 is the hash of the data recieved in the body response.
/// In case of an error, returns a string with an error message.
pub async fn fetch_configs() -> Result<(Vec<String>, String), String> {
    // use reqwest to GET https://www.vpngate.net/api/iphone/
    // parse the response (it is in CSV format) and return the config

    let response = match reqwest::get("https://www.vpngate.net/api/iphone/").await {
        Ok(response) => response,
        Err(e) => return Err(format!("Failed to get response: {}", e)),
    };

    // get text, gracefully handle error
    let response_text = match response.text().await {
        Ok(text) => text,
        Err(e) => return Err(format!("Failed to get response text: {}", e)),
    };

    // get hash of the response
    let hash = format!("{:x}", md5::compute(response_text.clone()));

    // remove the first two lines
    let response_text = response_text.lines().skip(2).collect::<Vec<&str>>().join("\n");

    let mut openvpn_configs = Vec::new();

    // iterate through the CSV and extract the OpenVPN configs
    for line in response_text.lines() {
        // if the line is just "*", skip it
        if line == "*" {
            continue;
        }
        // schema is #HostName,IP,Score,Ping,Speed,CountryLong,CountryShort,NumVpnSessions,Uptime,TotalUsers,TotalTraffic,LogType,Operator,Message,OpenVPN_ConfigData_Base64
        // we are interested in the last field

        let fields = line.split(",").collect::<Vec<&str>>();
        let openvpn_config = fields.last().unwrap();

        // decode the base64
        let openvpn_config = match base64::engine::general_purpose::STANDARD.decode(openvpn_config) {
            Ok(config) => config,
            Err(e) => return Err(format!("Failed to decode base64: {}", e)),
        };

        // convert to string
        let openvpn_config = match String::from_utf8(openvpn_config) {
            Ok(config) => config,
            Err(e) => return Err(format!("Failed to convert to string: {}", e)),
        };

        openvpn_configs.push(openvpn_config);
    }

    Ok((openvpn_configs, hash))

}


/// Returns the egress IP address being used by the current connection.
/// In case of an error, returns a string with an error message.
pub async fn get_ip() -> Result<String, String> {
    // use reqwest to GET https://vpngate.net
    // parse the response and return the IP address

    let response = match reqwest::Client::builder().danger_accept_invalid_certs(true).build().unwrap().get("http://api.ipify.org").send().await {
        Ok(response) => response,
        Err(e) => return Err(format!("Failed to get response: {:?}", e)),
    };

    let ip = match response.text().await {
        Ok(ip) => ip,
        Err(e) => return Err(format!("Failed to get response text: {}", e)),
    };

    Ok(ip.to_string())
}

/// Connect to the VPN server using the provided OpenVPN configuration.
/// `config` is a string containing the contents of the OpenVPN configuration file needed to connect to the VPN server.
/// In case of an error, returns a string with an error message.
pub async fn connect(index: u16, config: String) -> Result<String, String> {
    // write the config to a temporary file
    // use std::process::Command to run openvpn with the config file
    // return the result

    let temp_config = format!("./configs/{}.config", index);
    // write the contents of config to temp_config
    if let Err(e) = fs::create_dir_all("./configs").await {
        return Err(format!("Failed to create directory: {}", e));
    }
    
    let mut file = match fs::File::create(&temp_config).await {
        Ok(file) => file,
        Err(e) => return Err(format!("Failed to create config file: {}", e)),
    };
    match file.write(config.as_bytes()).await {
        Ok(_) => {},
        Err(e) => return Err(format!("Failed to write to config file: {}", e)),
    }

    let temp_log = format!("./logs/{}.log", index);
    if let Err(e) = fs::create_dir_all("./logs").await {
        return Err(format!("Failed to create directory: {}", e));
    }
    // clear the log file
    match fs::OpenOptions::new().write(true).truncate(true).open(&temp_log).await {
        Ok(mut file) => {
            match file.write_all(b"").await {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to clear log file: {}", e)),
            }
        },
        Err(e) => return Err(format!("Failed to open log file: {}", e)),
    }

    // Run openvpn without daemon mode, capturing its output
    let mut child = Command::new("sudo")
        .arg("openvpn")
        .arg("--config")
        .arg(temp_config)
        .arg("--data-ciphers")
        .arg("AES-128-CBC") // required for VPNgate connections to succeed on modern versions of OpenVPN
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run openvpn: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
   
    let mut buf_reader = BufReader::new(stdout);

    // Monitor the output from OpenVPN in real-time
    let mut buffer = Vec::new();
    loop {
        buf_reader.read_buf(&mut buffer).await.unwrap();

        if let Ok(line) = String::from_utf8(buffer.clone()) {
             // write the line to the log file
             match fs::OpenOptions::new().append(true).create(true).open(&temp_log).await {
                Ok(mut file) => {
                    match file.write_all(line.as_bytes()).await {
                        Ok(_) => {},
                        Err(e) => return Err(format!("Failed to write to log file: {}", e)),
                    }
                },
                Err(e) => return Err(format!("Failed to open log file: {}", e)),
            };
            if line.contains("Initialization Sequence Completed") {
                break;
            }
        }

        buffer.clear();
    }

    Ok(temp_log)
}

/// This function will handle terminating all OpenVPN processes.
pub async fn terminate_openvpn() -> Result<(), String> {
    let output = Command::new("sudo")
        .arg("killall")
        .arg("openvpn")
        .output()
        .await
        .map_err(|e| format!("Failed to run killall: {}", e))?;

    if !output.status.success() {
        return Err("Failed to terminate OpenVPN processes".into());
    }

    Ok(())
}

/// Test the VPN connection by connecting to the VPN server, getting the new IP address, and comparing it with the initial IP address.
/// `index` is the index of the VPN server in the configurations vector.
/// `config` is a string containing the contents of the OpenVPN configuration file needed to connect to the VPN server.
/// `initial_ip` is the initial IP address before connecting to the VPN.
/// In case of an error, returns a string with an error message.
pub async fn test_vpn(index: u16, config: String, initial_ip: String, max_stage_time: u64) -> Result<String, Box<dyn std::error::Error>> {
    // Connect to the VPN server
    match timeout(Duration::from_secs(max_stage_time), connect(index, config)).await {
        Ok(Ok(_)) => {
            print!(" connected!");
        },
        Ok(Err(e)) => {
            return Err(e.into());
        },
        Err(_) => {
            return Err("Connection to VPN server timed out".into());
        }
    }
    // Get the new IP address after connecting to the VPN
    let new_ip = match timeout(Duration::from_secs(max_stage_time), get_ip()).await {
        Ok(Ok(new_ip)) => {
            new_ip
        },
        Ok(Err(e)) => {
            return Err(e.into());
        },
        Err(_) => {
            return Err("Failed to get new IP address".into());
        }
    };

    if initial_ip == new_ip {
        return Err("IP address did not change after connecting to VPN".into());
    }

    // check that new_ip is in the format of an IP address
    let ip_regex = regex::Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap();
    if !ip_regex.is_match(&new_ip) {
        return Err("New IP address is not in the correct format".into());
    }

    println!(" Egress IP is {}", new_ip.clone().blue().bold());

    terminate_openvpn().await?;
    Ok(new_ip)
}