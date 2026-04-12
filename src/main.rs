mod config;
mod manager;

use clap::{Parser, Subcommand};
use config::parse_ssh_config;
use manager::TunnelManager;

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
    }
}
