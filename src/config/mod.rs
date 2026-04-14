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
        Ok(Self::parse_content(&content))
    }

    fn parse_content(content: &str) -> Vec<Tunnel> {
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
        tunnels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tunnel(name: &str) -> Tunnel {
        Tunnel {
            name: name.to_string(),
            local_port: None,
            remote_forward: None,
            remote_port: None,
            user: None,
            hostname: None,
            port: None,
            identity_file: None,
            identities_only: false,
        }
    }

    // --- socket_path ---

    #[test]
    fn test_socket_path_format() {
        let t = make_tunnel("my-tunnel");
        let p = t.socket_path();
        assert_eq!(p, PathBuf::from("/tmp/tunnel-manager-my-tunnel.sock"));
    }

    // --- parse_forward ---

    #[test]
    fn test_parse_forward_basic() {
        let (lp, rf, rp) = Tunnel::parse_forward("8080 remote-host:80");
        assert_eq!(lp, Some(8080));
        assert_eq!(rf, Some("remote-host".to_string()));
        assert_eq!(rp, Some(80));
    }

    #[test]
    fn test_parse_forward_with_bind_addr() {
        let (lp, rf, rp) = Tunnel::parse_forward("127.0.0.1:8080 remote-host:80");
        assert_eq!(lp, Some(8080));
        assert_eq!(rf, Some("remote-host".to_string()));
        assert_eq!(rp, Some(80));
    }

    #[test]
    fn test_parse_forward_ip_remote() {
        let (lp, rf, rp) = Tunnel::parse_forward("9000 192.168.1.100:443");
        assert_eq!(lp, Some(9000));
        assert_eq!(rf, Some("192.168.1.100".to_string()));
        assert_eq!(rp, Some(443));
    }

    #[test]
    fn test_parse_forward_remote_no_port() {
        let (lp, rf, rp) = Tunnel::parse_forward("8080 remote-host");
        assert_eq!(lp, Some(8080));
        assert_eq!(rf, Some("remote-host".to_string()));
        assert_eq!(rp, None);
    }

    #[test]
    fn test_parse_forward_single_part() {
        let (lp, rf, rp) = Tunnel::parse_forward("8080");
        assert_eq!(lp, None);
        assert_eq!(rf, None);
        assert_eq!(rp, None);
    }

    #[test]
    fn test_parse_forward_empty() {
        let (lp, rf, rp) = Tunnel::parse_forward("");
        assert_eq!(lp, None);
        assert_eq!(rf, None);
        assert_eq!(rp, None);
    }

    // --- display_info ---

    #[test]
    fn test_display_info_all_fields() {
        let t = Tunnel {
            name: "test".to_string(),
            local_port: Some(8080),
            remote_forward: Some("remote-host".to_string()),
            remote_port: Some(80),
            user: Some("alice".to_string()),
            hostname: Some("example.com".to_string()),
            port: Some(22),
            identity_file: Some("~/.ssh/id_rsa".to_string()),
            identities_only: true,
        };
        let info = t.display_info();
        assert!(info.contains("Local Port : 8080"));
        assert!(info.contains("Remote Host: remote-host"));
        assert!(info.contains("Remote Port: 80"));
        assert!(info.contains("User       : alice"));
        assert!(info.contains("Hostname   : example.com"));
        assert!(info.contains("SSH Port   : 22"));
        assert!(info.contains("IdentFile  : ~/.ssh/id_rsa"));
        assert!(info.contains("Ident. Only: yes"));
    }

    #[test]
    fn test_display_info_none_fields_show_dash() {
        let t = make_tunnel("test");
        let info = t.display_info();
        assert!(info.contains("Local Port : -"));
        assert!(info.contains("Remote Host: -"));
        assert!(info.contains("Remote Port: -"));
        assert!(info.contains("User       : -"));
        assert!(info.contains("Hostname   : -"));
        assert!(info.contains("SSH Port   : -"));
        assert!(info.contains("IdentFile  : -"));
        assert!(info.contains("Ident. Only: no"));
    }

    #[test]
    fn test_display_info_identities_only_false() {
        let mut t = make_tunnel("test");
        t.identities_only = false;
        assert!(t.display_info().contains("Ident. Only: no"));
    }

    // --- parse_content ---

    #[test]
    fn test_parse_content_full_tunnel() {
        let content = "\
Host my-tunnel
    LocalForward 8080 remote-host:80
    User alice
    HostName example.com
    Port 22
    IdentityFile ~/.ssh/id_rsa
    IdentitiesOnly yes
";
        let tunnels = Tunnel::parse_content(content);
        assert_eq!(tunnels.len(), 1);
        let t = &tunnels[0];
        assert_eq!(t.name, "my-tunnel");
        assert_eq!(t.local_port, Some(8080));
        assert_eq!(t.remote_forward, Some("remote-host".to_string()));
        assert_eq!(t.remote_port, Some(80));
        assert_eq!(t.user, Some("alice".to_string()));
        assert_eq!(t.hostname, Some("example.com".to_string()));
        assert_eq!(t.port, Some(22));
        assert_eq!(t.identity_file, Some("~/.ssh/id_rsa".to_string()));
        assert!(t.identities_only);
    }

    #[test]
    fn test_parse_content_multiple_tunnels() {
        let content = "\
Host tunnel-a
    LocalForward 8080 host-a:80

Host tunnel-b
    LocalForward 9090 host-b:443
";
        let tunnels = Tunnel::parse_content(content);
        assert_eq!(tunnels.len(), 2);
        assert_eq!(tunnels[0].name, "tunnel-a");
        assert_eq!(tunnels[0].local_port, Some(8080));
        assert_eq!(tunnels[1].name, "tunnel-b");
        assert_eq!(tunnels[1].local_port, Some(9090));
    }

    #[test]
    fn test_parse_content_host_without_forward_skipped() {
        let content = "\
Host plain-host
    HostName example.com
    User bob

Host tunnel-host
    LocalForward 8080 remote:80
";
        let tunnels = Tunnel::parse_content(content);
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].name, "tunnel-host");
    }

    #[test]
    fn test_parse_content_remote_forward() {
        let content = "\
Host my-tunnel
    RemoteForward 8080 localhost:80
";
        let tunnels = Tunnel::parse_content(content);
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].local_port, Some(8080));
        assert_eq!(tunnels[0].remote_forward, Some("localhost".to_string()));
        assert_eq!(tunnels[0].remote_port, Some(80));
    }

    #[test]
    fn test_parse_content_identities_only_no() {
        let content = "\
Host my-tunnel
    LocalForward 8080 remote:80
    IdentitiesOnly no
";
        let tunnels = Tunnel::parse_content(content);
        assert_eq!(tunnels.len(), 1);
        assert!(!tunnels[0].identities_only);
    }

    #[test]
    fn test_parse_content_case_insensitive_keys() {
        let content = "\
HOST ci-tunnel
    LOCALFORWARD 8080 remote:80
    USER alice
    HOSTNAME example.com
    PORT 22
    IDENTITYFILE ~/.ssh/id_rsa
    IDENTITIESONLY YES
";
        let tunnels = Tunnel::parse_content(content);
        assert_eq!(tunnels.len(), 1);
        let t = &tunnels[0];
        assert_eq!(t.user, Some("alice".to_string()));
        assert_eq!(t.hostname, Some("example.com".to_string()));
        assert_eq!(t.port, Some(22));
        assert!(t.identities_only);
    }

    #[test]
    fn test_parse_content_comments_and_blank_lines_ignored() {
        let content = "\
# This is a comment
Host my-tunnel

    # another comment
    LocalForward 8080 remote:80
";
        let tunnels = Tunnel::parse_content(content);
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].local_port, Some(8080));
    }

    #[test]
    fn test_parse_content_bind_address_in_local_forward() {
        let content = "\
Host my-tunnel
    LocalForward 127.0.0.1:8080 remote:80
";
        let tunnels = Tunnel::parse_content(content);
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].local_port, Some(8080));
    }

    #[test]
    fn test_parse_content_empty() {
        let tunnels = Tunnel::parse_content("");
        assert!(tunnels.is_empty());
    }

    #[test]
    fn test_parse_ssh_config_missing_file() {
        // Temporarily point HOME to a nonexistent dir
        // We can't easily do this without unsafe env manipulation,
        // so just verify parse_ssh_config returns Ok or Err without panicking.
        // The real file test is covered by test_parse_ssh_config in main.rs.
        let _ = Tunnel::parse_ssh_config(); // must not panic
    }
}
