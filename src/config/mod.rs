use regex::Regex;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Tunnel {
    pub name: String,
    pub forward: String,
}

pub fn parse_ssh_config() -> Result<Vec<Tunnel>, std::io::Error> {
    let home = std::env::var("HOME").expect("HOME not set");
    let path = PathBuf::from(home).join(".ssh/config");
    let content = fs::read_to_string(path)?;

    let mut tunnels = Vec::new();
    let mut current_host = None;

    let host_re = Regex::new(r"^Host\s+(.+)$").unwrap();
    let forward_re = Regex::new(r"^(LocalForward|RemoteForward)\s+(.+)$").unwrap();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(cap) = host_re.captures(line) {
            current_host = Some(cap[1].trim().to_string());
        } else if let Some(cap) = forward_re.captures(line) {
            if let Some(ref host) = current_host {
                tunnels.push(Tunnel {
                    name: host.clone(),
                    forward: cap[2].trim().to_string(),
                });
            }
        }
    }
    Ok(tunnels)
}
