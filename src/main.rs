mod config;
mod manager;
mod tui;
//mod tui;

use clap::{Parser, Subcommand};
use config::parse_ssh_config;
use manager::TunnelManager;
use tui::{TunnelInfo, run_tui};

#[derive(Parser)]
#[command(name = "tunnel-manager")]
#[command(about = "Manage SSH tunnels from ~/.ssh/config", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available tunnels from config
    List,
    /// Open a tunnel
    Open { name: String },
    /// Close a tunnel
    Close { name: String },
    /// Open TUI interface
    Tui,
}

fn main() {
    let cli = Cli::parse();
    let manager = TunnelManager::new();

    match cli.command {
        Commands::List => match parse_ssh_config() {
            Ok(tunnels) => {
                let active = manager.list_active();
                println!("Tunnels:");
                for t in tunnels {
                    let status = if active.contains(&t.name) {
                        "[ACTIVE]"
                    } else {
                        "[ ]"
                    };
                    println!("{} {}: {}", status, t.name, t.forward);
                }
            }
            Err(e) => eprintln!("Error reading config: {}", e),
        },
        Commands::Open { name } => match parse_ssh_config() {
            Ok(tunnels) => {
                if tunnels.iter().any(|t| t.name == name) {
                    match manager.open_tunnel(&name) {
                        Ok(_) => println!("Done"),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    eprintln!("Tunnel {} not found in config", name);
                }
            }
            Err(e) => eprintln!("Error reading config: {}", e),
        },
        Commands::Close { name } => match manager.close_tunnel(&name) {
            Ok(_) => println!("Done"),
            Err(e) => eprintln!("Error: {}", e),
        },
        Commands::Tui => match parse_ssh_config() {
            Ok(tunnels) => {
                let active = manager.list_active();
                // Convert config::Tunnel to tui::TunnelInfo
                let tunnel_infos: Vec<TunnelInfo> = tunnels
                    .into_iter()
                    .map(|t| {
                        let is_active = active.contains(&t.name);
                        // Try to parse forward string: "[host:]port [host:]port"
                        // Example: "127.0.0.1:8080 127.0.0.1:80" or "8080 80"
                        let mut remote_host = String::from("?");
                        let mut remote_port = 0u16;
                        let mut local_port = 0u16;
                        let parts: Vec<&str> = t.forward.split_whitespace().collect();
                        if parts.len() == 2 {
                            // LocalForward: local_bind remote_bind
                            let (local, remote) = (parts[0], parts[1]);
                            // Parse local
                            if let Some((_, port)) = local.rsplit_once(':') {
                                local_port = port.parse().unwrap_or(0);
                            } else {
                                local_port = local.parse().unwrap_or(0);
                            }
                            // Parse remote
                            if let Some((host, port)) = remote.rsplit_once(':') {
                                remote_host = host.to_string();
                                remote_port = port.parse().unwrap_or(0);
                            } else {
                                remote_port = remote.parse().unwrap_or(0);
                            }
                        }
                        TunnelInfo {
                            name: t.name,
                            active: is_active,
                            remote_host,
                            remote_port,
                            local_port,
                        }
                    })
                    .collect();
                let _ = run_tui(tunnel_infos);
            }
            Err(e) => eprintln!("Error: {}", e),
        },
    }
}
