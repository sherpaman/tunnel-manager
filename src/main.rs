mod config;
mod manager;
mod tui;

use clap::{Parser, Subcommand};
use config::Tunnel;
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

impl Default for Commands {
    fn default() -> Self {
        Commands::Tui
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => match Tunnel::parse_ssh_config() {
            Ok(tunnels) => {
                let active = Tunnel::list_active();
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
        Commands::Open { name } => match Tunnel::parse_ssh_config() {
            Ok(tunnels) => {
                if let Some(tunnel) = tunnels.iter().find(|t| t.name == name) {
                    match tunnel.open_tunnel() {
                        Ok(_) => println!("Done"),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    eprintln!("Tunnel {} not found in config", name);
                }
            }
            Err(e) => eprintln!("Error reading config: {}", e),
        },
        Commands::Close { name } => {
            let tunnel = Tunnel {
                name,
                forward: String::new(),
                user: None,
                hostname: None,
                port: None,
                identity_file: None,
            };
            match tunnel.close_tunnel() {
                Ok(_) => println!("Done"),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Tui => match Tunnel::parse_ssh_config() {
            Ok(tunnels) => {
                let active = Tunnel::list_active();
                let tunnel_infos: Vec<TunnelInfo> = tunnels
                    .into_iter()
                    .map(|t| {
                        let is_active = active.contains(&t.name);
                        let mut local_port = 0u16;
                        let parts: Vec<&str> = t.forward.split_whitespace().collect();
                        if parts.len() == 2 {
                            let local = parts[0];
                            if let Some((_, port)) = local.rsplit_once(':') {
                                local_port = port.parse().unwrap_or(0);
                            } else {
                                local_port = local.parse().unwrap_or(0);
                            }
                        }
                        TunnelInfo {
                            tunnel: t,
                            active: is_active,
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

#[test]
fn test_is_active_false_for_nonexistent() {
    let tunnel = Tunnel {
        name: "definitely_nonexistent_tunnel".to_string(),
        forward: String::new(),
        user: None,
        hostname: None,
        port: None,
        identity_file: None,
    };
    assert!(!tunnel.is_active());
}

#[test]
fn test_open_and_close_tunnel() {
    // This test assumes you have a Host test-tunnel in your ~/.ssh/config
    let tunnels = Tunnel::parse_ssh_config().expect("Failed to parse config");
    let tunnel = tunnels.first().expect("No tunnels in config");
    // Try to close first in case it's open
    let _ = tunnel.close_tunnel();
    assert!(!tunnel.is_active());
    // Open
    let open_result = tunnel.open_tunnel();
    assert!(open_result.is_ok(), "Failed to open: {:?}", open_result);
    assert!(tunnel.is_active());
    // Close
    let close_result = tunnel.close_tunnel();
    assert!(close_result.is_ok(), "Failed to close: {:?}", close_result);
    assert!(!tunnel.is_active());
}

#[test]
fn test_list_active() {
    let active = Tunnel::list_active();
    // Should not panic, and should be a Vec
    assert!(active.is_empty() || active.iter().all(|n| !n.is_empty()));
}

#[test]
fn test_parse_ssh_config() {
    let tunnels = Tunnel::parse_ssh_config();
    assert!(tunnels.is_ok());
    let tunnels = tunnels.unwrap();
    for t in tunnels {
        assert!(!t.name.is_empty());
        assert!(!t.forward.is_empty());
    }
}
