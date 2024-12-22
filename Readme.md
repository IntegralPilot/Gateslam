# Gateslam

**Gateslam 0.1.0 by IntegralPilot** - Easily discover the current VPNGate egress IP addresses. Intended for webhosts to manage abuse.

## Overview

Gateslam is a tool for automatically fetching VPN configurations from [VPNGate](https://www.vpngate.net/), connecting to these VPN servers using OpenVPN, and retrieving the new egress IP address after connection. The tool can also update a MediaWiki page with the discovered IP addresses.

_Note: This is useful to the operators of web servers who want to know when a user is using VPNgate to connect to their server. It is **not** helpful in any way to schools, companies or countries wanting to block access to VPNgate (if this is you, don't use this because it won't block anything), because it detects the IP addresses used for VPNgate to talk to servers, **not** the IP addresses used for users to talk to VPNgate - they are a different set._

This project supports:
- Fetching VPN configurations from VPNGate in OpenVPN format.
- Connecting to VPNGate servers and verifying that the connection is successful.
- Determining and comparing the egress IP address before and after connection to confirm success.
- Updating a MediaWiki page with the discovered egress IP addresses, if configured with MediaWiki support.

## "Egress" IP address, what is that?

When using a modern VPN (Virtual Private Network) like VPNGate, there are two types of IP addresses involved in the connection process: **ingress** and **egress** IP addresses.

- **Ingress IP**: This is the IP address that your device connects to when establishing a VPN connection. It is provided by the VPN service and represents the server your computer uses to create the encrypted tunnel. It is often public and shared, allowing you to establish a secure connection to the VPN.
  
- **Egress IP**: This is the IP address that the VPN server uses to connect to external websites or services on behalf of your device. This IP is seen by the destination (like Wikipedia) as the origin of the request. The egress IP may differ from the ingress IP, and this is crucial because it is the egress IP that can reveal the actual VPN’s interaction with external systems.

## Features

1. **Fetch VPN Configurations**: Automatically fetches the list of available OpenVPN configurations from VPNGate.
2. **Test VPN Connections**: Attempts to connect to each VPN server, retrieves the new egress IP, and compares it to the original IP.
3. **MediaWiki Integration** (Optional): Posts the discovered egress IP addresses to a MediaWiki page (such as Wikipedia).
4. **Logging**: Logs all connection attempts and outputs the results to a log file for each server.

Note: Enabling MediaWiki integration requires enabling the optional cargo feature `mediawiki`.

## Prerequisites

- **Unix**: This software assumes a Unix environment, such as Linux or macOS.
- **OpenVPN**: Make sure OpenVPN is installed on your machine.
- **Rust**: The project uses Rust as the primary programming language. Install Rust from [rust-lang.org](https://www.rust-lang.org/).
- **Root**: Some operations, such as starting the VPN connection, require root privileges.
- **MediaWiki Integration** (Optional): If using the MediaWiki feature, configure access to a MediaWiki site with a bot user.
- - The bot should have the username `MolecularBot` and you need to create the page `User:MolecularBot/IPData.json`.
- - You also need to setup `mwbot` by creating and populating `~/.config/mwbot.toml` with API urls and login credentials. See [that project's page](https://crates.io/crates/mwbot) for instructions.
