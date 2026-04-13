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

impl Default for Commands {
    fn default() -> Self {
        Commands::Tui
    }
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


// #[test]
// fn test_socket_path() {
//     let path = TunnelManager::socket_path("test");
//     assert!(path.ends_with("tunnel-manager-test.sock"));
// }

#[test]
fn test_is_active_false_for_nonexistent() {
    let manager = TunnelManager::new();
    assert!(!manager.is_active("definitely_nonexistent_tunnel"));
}

#[test]
fn test_open_and_close_tunnel() {
    // This test assumes you have a Host test-tunnel in your ~/.ssh/config
    let manager = TunnelManager::new();
    let tunnels = parse_ssh_config().expect("Failed to parse config");
    let name = tunnels.first().expect("No tunnels in config").name.clone();
    // Try to close first in case it's open
    let _ = manager.close_tunnel(&name);
    assert!(!manager.is_active(&name));
    // Open
    let open_result = manager.open_tunnel(&name);
    assert!(open_result.is_ok(), "Failed to open: {:?}", open_result);
    assert!(manager.is_active(&name));
    // Close
    let close_result = manager.close_tunnel(&name);
    assert!(close_result.is_ok(), "Failed to close: {:?}", close_result);
    assert!(!manager.is_active(&name));
}

#[test]
fn test_list_active() {
    let manager = TunnelManager::new();
    let active = manager.list_active();
    // Should not panic, and should be a Vec
    assert!(active.is_empty() || active.iter().all(|n| !n.is_empty()));
}

#[test]
fn test_parse_ssh_config() {
    let tunnels = parse_ssh_config();
    assert!(tunnels.is_ok());
    let tunnels = tunnels.unwrap();
    for t in tunnels {
        assert!(!t.name.is_empty());
        assert!(!t.forward.is_empty());
    }
}

