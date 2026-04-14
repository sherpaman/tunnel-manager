use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Tunnel {
    pub name: String,
    pub local_port: Option<u16>,
    pub remote_forward: Option<String>,
    pub remote_port: Option<u16>,
    pub user: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub identities_only: bool,
}

impl Tunnel {
    fn socket_path(&self) -> PathBuf {
        PathBuf::from(format!("/tmp/tunnel-manager-{}.sock", self.name))
    }

    /// Parse a LocalForward/RemoteForward value into (local_port, remote_host, remote_port).
    /// Format: "[bind_addr:]local_port remote_host:remote_port"
    fn parse_forward(raw: &str) -> (Option<u16>, Option<String>, Option<u16>) {
        let parts: Vec<&str> = raw.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return (None, None, None);
        }
        let local_port = if let Some((_, p)) = parts[0].rsplit_once(':') {
            p.parse().ok()
        } else {
            parts[0].parse().ok()
        };
        let (remote_forward, remote_port) = if let Some((host, port)) = parts[1].rsplit_once(':') {
            (Some(host.to_string()), port.parse().ok())
        } else {
            (Some(parts[1].to_string()), None)
        };
        (local_port, remote_forward, remote_port)
    }

    pub fn display_info(&self) -> String {
        format!(
            "Local Port : {}\nRemote Host: {}\nRemote Port: {}\nUser       : {}\nHostname   : {}\nSSH Port   : {}\nIdentFile  : {}\nIdent. Only: {}",
            self.local_port.map(|p| p.to_string()).as_deref().unwrap_or("-"),
            self.remote_forward.as_deref().unwrap_or("-"),
            self.remote_port.map(|p| p.to_string()).as_deref().unwrap_or("-"),
            self.user.as_deref().unwrap_or("-"),
            self.hostname.as_deref().unwrap_or("-"),
            self.port.map(|p| p.to_string()).as_deref().unwrap_or("-"),
            self.identity_file.as_deref().unwrap_or("-"),
            if self.identities_only { "yes" } else { "no" },
        )
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
        let mut identities_only = false;
        let mut local_port: Option<u16> = None;
        let mut remote_forward: Option<String> = None;
        let mut remote_port: Option<u16> = None;

        let host_re = Regex::new(r"(?i)^Host\s+(.+)$").unwrap();
        let user_re = Regex::new(r"(?i)^User\s+(.+)$").unwrap();
        let hostname_re = Regex::new(r"(?i)^Hostname\s+(.+)$").unwrap();
        let port_re = Regex::new(r"(?i)^Port\s+(\d+)$").unwrap();
        let identity_file_re = Regex::new(r"(?i)^IdentityFile\s+(.+)$").unwrap();
        let identities_only_re = Regex::new(r"(?i)^IdentitiesOnly\s+(.+)$").unwrap();
        let forward_re = Regex::new(r"(?i)^(LocalForward|RemoteForward)\s+(.+)$").unwrap();

        let flush = |tunnels: &mut Vec<Tunnel>,
                         host: &Option<String>,
                         local_port: Option<u16>,
                         remote_forward: Option<String>,
                         remote_port: Option<u16>,
                         user: Option<String>,
                         hostname: Option<String>,
                         port: Option<u16>,
                         identity_file: Option<String>,
                         identities_only: bool| {
            if let Some(name) = host {
                if local_port.is_some() || remote_forward.is_some() {
                    tunnels.push(Tunnel {
                        name: name.clone(),
                        local_port,
                        remote_forward,
                        remote_port,
                        user,
                        hostname,
                        port,
                        identity_file,
                        identities_only,
                    });
                }
            }
        };

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(cap) = host_re.captures(line) {
                flush(
                    &mut tunnels,
                    &current_host,
                    local_port,
                    remote_forward.clone(),
                    remote_port,
                    user.clone(),
                    hostname.clone(),
                    port,
                    identity_file.clone(),
                    identities_only,
                );
                current_host = Some(cap[1].trim().to_string());
                user = None;
                hostname = None;
                port = None;
                identity_file = None;
                identities_only = false;
                local_port = None;
                remote_forward = None;
                remote_port = None;
            } else if let Some(cap) = user_re.captures(line) {
                user = Some(cap[1].trim().to_string());
            } else if let Some(cap) = hostname_re.captures(line) {
                hostname = Some(cap[1].trim().to_string());
            } else if let Some(cap) = port_re.captures(line) {
                port = cap[1].parse::<u16>().ok();
            } else if let Some(cap) = identity_file_re.captures(line) {
                identity_file = Some(cap[1].trim().to_string());
            } else if let Some(cap) = identities_only_re.captures(line) {
                identities_only = cap[1].trim().eq_ignore_ascii_case("yes");
            } else if let Some(cap) = forward_re.captures(line) {
                let (lp, rf, rp) = Self::parse_forward(cap[2].trim());
                local_port = lp;
                remote_forward = rf;
                remote_port = rp;
            }
        }
        flush(
            &mut tunnels,
            &current_host,
            local_port,
            remote_forward,
            remote_port,
            user,
            hostname,
            port,
            identity_file,
            identities_only,
        );
        Ok(tunnels)
    }
}
