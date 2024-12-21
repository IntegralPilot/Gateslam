use colored::Colorize;
use gateslam::{mediawiki::{IPDataEntry, IPDataType}, terminate_openvpn, test_vpn};
use mwbot::Page;
use std::io::{self, Write};
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    run().await.await;
}

async fn run() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    let configurations: Vec<String>;
    println!("{} {} by {} - Discover VPNGate egress IP addresses.", "Gateslam".blue().bold(), "0.1.0".blue(), "IntegralPilot".bold());
    let bot;
    let mut page: Option<Page> = None;
    let mut current_data: Vec<IPDataEntry> = Vec::new();
    if cfg!(feature = "mediawiki") {
        print!("   {} to MediaWiki...", "Connecting".green().bold());
        io::stdout().flush().unwrap();
        bot = match mwbot::Bot::from_default_config().await {
            Ok(bot) => bot,
            Err(e) => {
                println!();
                eprintln!("      {} to connect to MediaWiki: {}", "Failed".red().bold(), e);
                std::process::exit(-1);
            },
        };
        page = match bot.page("User:MolecularBot/IPData.json") {
            Ok(page) => Some(page),
            Err(e) => {
                println!();
                eprintln!("      {} to get IPData page: {}", "Failed".red().bold(), e);
                std::process::exit(-1);
            },
        };
        let wikitext = match page.clone().unwrap().wikitext().await {
            Ok(data) => data.to_string(),
            Err(e) => {
                println!();
                eprintln!("      {} to get IPData wikitext: {}", "Failed".red().bold(), e);
                std::process::exit(-1);
            }
        };
        // parse it as a JSON array of IPDataEntry
        current_data = match serde_json::from_str(&wikitext) {
            Ok(data) => {
                println!(" connected!");
                data
            },
            Err(e) => {
                println!();
                eprintln!("      {} to parse IPData JSON: {}", "Failed".red().bold(), e);
                std::process::exit(-1);
            }
        };
    }

    let hash: String;
    print!("   {} configuration for each VPNGate server... ", "Retrieving".green().bold());
    io::stdout().flush().unwrap();  // Manually flush the buffer to ensure the above message is printed before the fetch_configs() function is called.
    match gateslam::fetch_configs().await {
        Ok(configs) => {
            hash = configs.1;
            configurations = configs.0.clone();
            println!("found {} {}!", configs.0.len().to_string().blue().bold(), "servers".blue());
        },
        Err(e) => {
            println!();
            eprintln!("     {} to fetch configs: {}", "Failed".red().bold(), e); 
            std::process::exit(1);
        },
    }
    
    // print the goal rate required to test each server in 1 hour
    let duration_secs = (60 * 60) / configurations.len() as u64;
    println!("   Each server will be tested for {} seconds.", duration_secs.to_string().blue().bold());

    let initial_ip: String;

    print!("   {} inital IP address...", "Determining".green().bold());
    io::stdout().flush().unwrap();
    match gateslam::get_ip().await {
        Ok(ip) => {
            println!(" it's {}!", ip.blue().bold());
            initial_ip = ip;
        },
        Err(e) => {
            println!();
            eprintln!("      {} to get initial IP: {}", "Failed".red().bold(), e);
            std::process::exit(2);
        },
    }

    // tell the user they may be asked for their password as openvpn requires sudo
    println!("   You may be asked for your password to run OpenVPN!");
    let mut index = 0;
    for config in configurations.clone() {
        print!("   {} to connect to VPN server {}...", "Attempting".green().bold(), index.to_string().blue().bold());
        io::stdout().flush().unwrap();
        match test_vpn(index, config.clone(), initial_ip.clone(), duration_secs).await {
            Ok(ip) => {
                if cfg!(feature = "mediawiki") {
                    // see if the IP is found in the current_data
                    if current_data.iter().any(|entry| entry.ip == ip.clone()) {
                        // capture the number of sightings for the IP
                        let sightings = current_data.iter().find(|entry| entry.ip == ip.clone()).unwrap().sightings;
                        // remove the entry from current_data
                        current_data = current_data.iter().filter(|entry| entry.ip != ip.clone()).cloned().collect();
                        // get the current unix timestamp in seconds
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                        // create a new entry
                        let new_entry = IPDataEntry {
                            ip: ip.clone(),
                            type_: IPDataType::ConfimedVpngateEgress,
                            last_sighting: now,
                            sightings: sightings + 1,
                        };
                        // add the new entry to the current_data
                        current_data.push(new_entry);
                        // update the page
                        let new_data = serde_json::to_string(&current_data).unwrap();
                        match page.clone().unwrap().save(&new_data, &mwbot::SaveOptions::summary(format!("Update listing for {} - new sighting", ip.clone()).as_str())).await {
                            Ok(_) => println!("      {} in IPData.json", "Updated".green().bold()),
                            Err(e) => eprintln!("      {} in IPData.json: {}", "Failed".red().bold(), e),
                        }
                    } else {
                        // insert the new entry at the end
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                        let new_entry = IPDataEntry {
                            ip: ip.clone(),
                            type_: IPDataType::ConfimedVpngateEgress,
                            last_sighting: now,
                            sightings: 1,
                        };
                        current_data.push(new_entry);
                        // update the page
                        let new_data = serde_json::to_string(&current_data).unwrap();
                        match page.clone().unwrap().save(&new_data, &mwbot::SaveOptions::summary(format!("Create listing for {} - novel sighting", ip).as_str())).await {
                            Ok(_) => println!("      {} in IPData.json", "Created".green().bold()),
                            Err(e) => eprintln!("      {} in IPData.json: {}", "Failed".red().bold(), e),
                        }

                    }
                }

            },
            Err(e) => {
                println!();
                eprintln!("      {} to connect to VPN: {}", "Failed".red().bold(), e);
            }
        }
        index += 1;
    }
    // confirm all VPN sessions have been terminated
    match terminate_openvpn().await {
        Ok(_) => println!("   {} all OpenVPN sessions.", "Terminated".green().bold()),
        Err(_) => {
            /* there were none to terminate, means every process was correctly terminated in main loop. */
        }
    }
    // now re-fetch the config every 5 minutes, and if the hash changes, call run() again
    loop {
        match gateslam::fetch_configs().await {
            Ok(new_configs) => {
                let new_hash = new_configs.1;
                if new_hash != hash {
                    println!("   {} - new configuration detected, restarting...", "Change detected".yellow().bold());
                    Box::pin(run()).await.await;  // Call run() again if the hash changes
                } else {
                    println!("   {} - no configuration change detected.", "No change".green().bold());
                }
            },
            Err(e) => {
                eprintln!("   {} to re-fetch configs: {}", "Failed".red().bold(), e);
            },
        }
        tokio::time::sleep(Duration::from_secs(60 * 5)).await; 
    }
}