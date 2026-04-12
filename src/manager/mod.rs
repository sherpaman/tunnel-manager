use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct TunnelManager;

impl TunnelManager {
    pub fn new() -> Self {
        Self
    }

    fn socket_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/tunnel-manager-{}.sock", name))
    }

    pub fn open_tunnel(&self, name: &str) -> Result<(), String> {
        if self.is_active(name) {
            return Err(format!("Tunnel {} already running", name));
        }

        let socket = Self::socket_path(name);

        // ssh -N -f -S <socket> <name>
        // -N: Do not execute remote command
        // -f: Request ssh to go to background
        // -S: Control socket for connection sharing
        let output = Command::new("ssh")
            .arg("-N")
            .arg("-f")
            .arg("-M")
            .arg("-S")
            .arg(&socket)
            .arg("-o ExitOnForwardFailure=yes")
            .arg(name)
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(format!(
                "SSH failed to open tunnel [{}]:\n{}",
                name,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        println!("{:?}", output);
        println!(
            "Opened tunnel {} with control socket at {:?}",
            name, socket
        );
        println!("Use 'tunnel-manager list' to see active tunnels.");

        Ok(())
    }

    pub fn close_tunnel(&self, name: &str) -> Result<(), String> {
        let socket = Self::socket_path(name);

        // ssh -S <socket> -O exit <name>
        let output = Command::new("ssh")
            .arg("-S")
            .arg(&socket)
            .arg("-O")
            .arg("exit")
            .arg(name)
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            // Cleanup socket file if it still exists
            let _ = fs::remove_file(socket);
            println!("Closed tunnel {}", name);
            Ok(())
        } else {
            Err(format!(
                "Failed to close tunnel {}: {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub fn is_active(&self, name: &str) -> bool {
        let socket = Self::socket_path(name);
        if !socket.exists() {
            return false;
        }

        // ssh -S <socket> -O check <name>
        let output = Command::new("ssh")
            .arg("-S")
            .arg(&socket)
            .arg("-O")
            .arg("check")
            .arg(name)
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    true
                } else {
                    // If the check fails, remove the stale socket file
                    let _ = fs::remove_file(&socket);
                    false
                }
            },
            Err(_) => {
                let _ = fs::remove_file(&socket);
                false
            },
        }
    }

    pub fn list_active(&self) -> Vec<String> {
        let mut active = Vec::new();
        if let Ok(entries) = fs::read_dir("/tmp") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name.starts_with("tunnel-manager-") && file_name.ends_with(".sock") {
                        let name = file_name
                            .strip_prefix("tunnel-manager-")
                            .and_then(|s| s.strip_suffix(".sock"))
                            .unwrap_or("");
                        if !name.is_empty() && self.is_active(name) {
                            active.push(name.to_string());
                        }
                    }
                }
            }
        }
        active
    }
}
