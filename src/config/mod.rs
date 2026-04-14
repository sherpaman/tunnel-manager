use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Tunnel {
    pub name: String,
    pub forward: String,
    pub user: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
}

impl Tunnel {
    fn socket_path(&self) -> PathBuf {
        PathBuf::from(format!("/tmp/tunnel-manager-{}.sock", self.name))
    }

    pub fn open_tunnel(&self) -> Result<(), String> {
        if self.is_active() {
            return Err(format!("Tunnel {} already running", self.name));
        }
        let socket = self.socket_path();
        let output = Command::new("ssh")
            .arg("-N")
            .arg("-f")
            .arg("-M")
            .arg("-S")
            .arg(&socket)
            .arg("-o ExitOnForwardFailure=yes")
            .arg(&self.name)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "SSH failed to open tunnel [{}]:\n{}",
                self.name,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    pub fn close_tunnel(&self) -> Result<(), String> {
        let socket = self.socket_path();
        let output = Command::new("ssh")
            .arg("-S")
            .arg(&socket)
            .arg("-O")
            .arg("exit")
            .arg(&self.name)
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            let _ = fs::remove_file(socket);
            Ok(())
        } else {
            Err(format!(
                "Failed to close tunnel {}: {}",
                self.name,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub fn is_active(&self) -> bool {
        let socket = self.socket_path();
        if !socket.exists() {
            return false;
        }
        let output = Command::new("ssh")
            .arg("-S")
            .arg(&socket)
            .arg("-O")
            .arg("check")
            .arg(&self.name)
            .output();
        match output {
            Ok(out) => {
                if out.status.success() {
                    true
                } else {
                    let _ = fs::remove_file(&socket);
                    false
                }
            }
            Err(_) => {
                let _ = fs::remove_file(&socket);
                false
            }
        }
    }

    pub fn list_active() -> Vec<String> {
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
                        if !name.is_empty() {
                            let output = Command::new("ssh")
                                .arg("-S")
                                .arg(&path)
                                .arg("-O")
                                .arg("check")
                                .arg(name)
                                .output();
                            match output {
                                Ok(out) => {
                                    if out.status.success() {
                                        active.push(name.to_string());
                                    } else {
                                        let _ = fs::remove_file(&path);
                                    }
                                }
                                Err(_) => {
                                    let _ = fs::remove_file(&path);
                                }
                            }
                        }
                    }
                }
            }
        }
        active
    }

    pub fn parse_ssh_config() -> Result<Vec<Tunnel>, std::io::Error> {
    let home = std::env::var("HOME").expect("HOME not set");
    let path = PathBuf::from(home).join(".ssh/config");
    let content = fs::read_to_string(path)?;

    let mut tunnels = Vec::new();
    let mut current_host: Option<String> = None;
    let mut user: Option<String> = None;
    let mut hostname: Option<String> = None;
    let mut port: Option<u16> = None;
    let mut identity_file: Option<String> = None;
    let mut forward: Option<String> = None;

    let host_re = Regex::new(r"^Host\s+(.+)$").unwrap();
    let user_re = Regex::new(r"^User\s+(.+)$").unwrap();
    let hostname_re = Regex::new(r"^Hostname\s+(.+)$").unwrap();
    let port_re = Regex::new(r"^Port\s+(\d+)$").unwrap();
    let identity_file_re = Regex::new(r"^IdentityFile\s+(.+)$").unwrap();
    let forward_re = Regex::new(r"^(LocalForward|RemoteForward)\s+(.+)$").unwrap();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(cap) = host_re.captures(line) {
            if let Some(ref host) = current_host {
                if let Some(ref fwd) = forward {
                    tunnels.push(Tunnel {
                        name: host.clone(),
                        forward: fwd.clone(),
                        user: user.clone(),
                        hostname: hostname.clone(),
                        port,
                        identity_file: identity_file.clone(),
                    });
                }
            }
            current_host = Some(cap[1].trim().to_string());
            user = None;
            hostname = None;
            port = None;
            identity_file = None;
            forward = None;
        } else if let Some(cap) = user_re.captures(line) {
            user = Some(cap[1].trim().to_string());
        } else if let Some(cap) = hostname_re.captures(line) {
            hostname = Some(cap[1].trim().to_string());
        } else if let Some(cap) = port_re.captures(line) {
            port = cap[1].parse::<u16>().ok();
        } else if let Some(cap) = identity_file_re.captures(line) {
            identity_file = Some(cap[1].trim().to_string());
        } else if let Some(cap) = forward_re.captures(line) {
            forward = Some(cap[2].trim().to_string());
        }
    }
    if let Some(ref host) = current_host {
        if let Some(ref fwd) = forward {
            tunnels.push(Tunnel {
                name: host.clone(),
                forward: fwd.clone(),
                user,
                hostname,
                port,
                identity_file,
            });
        }
    }
        Ok(tunnels)
    }
}
