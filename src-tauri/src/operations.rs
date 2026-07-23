use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationalContext {
    pub kind: String,
    pub environment: String,
    pub shell: Option<String>,
    pub hostnames: Vec<String>,
    pub ip_addresses: Vec<String>,
    pub services: Vec<String>,
    pub technologies: Vec<String>,
}

impl OperationalContext {
    pub fn is_operational(&self) -> bool {
        !self.kind.is_empty()
    }

    pub fn tags(&self) -> Vec<String> {
        let mut tags = vec!["Operations".to_string()];
        tags.extend(self.services.clone());
        tags.extend(self.technologies.clone());
        if !self.hostnames.is_empty() {
            tags.push("Host".to_string());
        }
        if !self.ip_addresses.is_empty() {
            tags.push("IP Address".to_string());
        }
        if self.kind == "Incident" {
            tags.push("Incident".to_string());
        }
        tags.sort();
        tags.dedup();
        tags
    }

    pub fn summary(&self) -> String {
        let subject = self
            .services
            .first()
            .or_else(|| self.technologies.first())
            .cloned()
            .unwrap_or_else(|| "Linux environment".to_string());

        match self.kind.as_str() {
            "Incident" => format!("Operational incident signal related to {subject}."),
            "Configuration" => format!("Operational configuration captured for {subject}."),
            "Terminal Command" => format!("Linux operational command for {subject}."),
            _ => "Operational memory captured locally.".to_string(),
        }
    }
}

pub fn analyze(content: &str, shell: Option<&str>) -> OperationalContext {
    let lower = content.to_lowercase();
    let command = command_name(content);
    let is_incident = [
        "permission denied",
        "connection refused",
        "failed",
        "fatal",
        "panic",
        "exception",
        "selinux denied",
        "avc: denied",
        "error",
    ]
    .iter()
    .any(|signal| lower.contains(signal));
    let is_configuration = [
        "server {",
        "listen ",
        "apiversion:",
        "kind:",
        "[unit]",
        "[service]",
        "allowusers",
        "sshd_config",
        ".conf",
        "firewall-cmd --permanent",
    ]
    .iter()
    .any(|signal| lower.contains(signal));
    let is_command = command
        .as_deref()
        .map(is_operational_command)
        .unwrap_or(false);
    let technologies = technologies(&lower);
    let services = services(&lower);

    if !is_incident
        && !is_configuration
        && !is_command
        && technologies.is_empty()
        && services.is_empty()
    {
        return OperationalContext::default();
    }

    let kind = if is_incident {
        "Incident"
    } else if is_configuration {
        "Configuration"
    } else {
        "Terminal Command"
    };

    OperationalContext {
        kind: kind.to_string(),
        environment: "Linux Operations".to_string(),
        shell: shell.map(ToString::to_string),
        hostnames: hostnames(content),
        ip_addresses: ip_addresses(content),
        services,
        technologies,
    }
}

pub fn is_sensitive_command(command: &str) -> bool {
    let lower = command.to_lowercase();
    let trimmed = lower.trim_start();
    trimmed.starts_with("export ")
        || [
            "password",
            "passwd",
            "secret",
            "token",
            "api_key",
            "apikey",
            "authorization",
            "private_key",
            "sshpass",
            "kubectl create secret",
            "aws configure",
            ".pem",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

pub fn is_low_signal_command(command: &str) -> bool {
    matches!(
        command_name(command).as_deref(),
        None | Some("clear" | "exit" | "history" | "pwd" | "ls" | "cd")
    )
}

fn command_name(content: &str) -> Option<String> {
    content
        .trim_start()
        .trim_start_matches("sudo ")
        .split_whitespace()
        .next()
        .map(|command| command.rsplit('/').next().unwrap_or(command).to_lowercase())
}

fn is_operational_command(command: &str) -> bool {
    matches!(
        command,
        "systemctl"
            | "journalctl"
            | "service"
            | "firewall-cmd"
            | "iptables"
            | "nft"
            | "ss"
            | "ip"
            | "dig"
            | "nslookup"
            | "host"
            | "ssh"
            | "scp"
            | "rsync"
            | "kubectl"
            | "helm"
            | "docker"
            | "podman"
            | "ansible"
            | "ansible-playbook"
            | "psql"
            | "mysql"
            | "mariadb"
            | "sqlite3"
            | "nginx"
            | "httpd"
            | "apachectl"
            | "setenforce"
            | "getenforce"
            | "semanage"
            | "audit2why"
            | "openssl"
            | "git"
    )
}

fn technologies(lower: &str) -> Vec<String> {
    labels_for(
        lower,
        &[
            ("kubernetes", "Kubernetes"),
            ("kubectl", "Kubernetes"),
            ("helm", "Helm"),
            ("docker", "Docker"),
            ("podman", "Podman"),
            ("ansible", "Ansible"),
            ("selinux", "SELinux"),
            ("firewall", "Firewall"),
            ("iptables", "Firewall"),
            ("nft ", "Firewall"),
            ("ssh", "SSH"),
            ("postgres", "PostgreSQL"),
            ("psql", "PostgreSQL"),
            ("mysql", "MySQL"),
            ("mariadb", "MariaDB"),
            ("git", "Git"),
            ("dns", "DNS"),
        ],
    )
}

fn services(lower: &str) -> Vec<String> {
    labels_for(
        lower,
        &[
            ("nginx", "Nginx"),
            ("httpd", "Apache HTTPD"),
            ("apache", "Apache HTTPD"),
            ("postgresql", "PostgreSQL"),
            ("postgres", "PostgreSQL"),
            ("mysql", "MySQL"),
            ("mariadb", "MariaDB"),
            ("sshd", "OpenSSH"),
            ("kubelet", "Kubelet"),
            ("docker", "Docker"),
            ("firewalld", "Firewalld"),
            ("named", "BIND DNS"),
        ],
    )
}

fn labels_for(lower: &str, labels: &[(&str, &str)]) -> Vec<String> {
    let mut found = labels
        .iter()
        .filter(|(needle, _)| lower.contains(needle))
        .map(|(_, label)| (*label).to_string())
        .collect::<Vec<_>>();
    found.sort();
    found.dedup();
    found
}

fn hostnames(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    for token in content.split_whitespace() {
        let token = token.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric()
                && character != '.'
                && character != '-'
                && character != '_'
                && character != '@'
        });
        let host = token
            .rsplit_once('@')
            .map(|(_, host)| host)
            .unwrap_or(token);
        let valid = host.len() <= 253
            && host.contains('.')
            && host.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 63
                    && part
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '-')
            });
        if valid && host.parse::<Ipv4Addr>().is_err() && host.parse::<Ipv6Addr>().is_err() {
            names.push(host.to_string());
        }
    }

    let words = content.split_whitespace().collect::<Vec<_>>();
    if matches!(words.first(), Some(&"ssh") | Some(&"scp") | Some(&"rsync")) {
        if let Some(candidate) = words.iter().skip(1).find(|word| !word.starts_with('-')) {
            let host = candidate
                .trim_matches(|character: char| {
                    !character.is_ascii_alphanumeric()
                        && character != '-'
                        && character != '_'
                        && character != '@'
                })
                .rsplit_once('@')
                .map(|(_, host)| host)
                .unwrap_or(candidate);
            if host.len() <= 253
                && host.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
            {
                names.push(host.to_string());
            }
        }
    }
    names.sort();
    names.dedup();
    names.truncate(8);
    names
}

fn ip_addresses(content: &str) -> Vec<String> {
    let mut addresses = content
        .split(|character: char| {
            character.is_whitespace() || matches!(character, ',' | ';' | '(' | ')' | '[' | ']')
        })
        .filter_map(|token| {
            let value = token.trim_matches(|character: char| character == ':' || character == '.');
            if value.parse::<Ipv4Addr>().is_ok() || value.parse::<Ipv6Addr>().is_ok() {
                Some(value.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();
    addresses.truncate(8);
    addresses
}

#[cfg(test)]
mod tests {
    use super::{analyze, is_sensitive_command};

    #[test]
    fn classifies_linux_commands_with_context() {
        let context = analyze(
            "ssh admin@web-01.ops.example.com systemctl restart nginx",
            Some("Bash"),
        );
        assert_eq!(context.kind, "Terminal Command");
        assert!(context.services.contains(&"Nginx".to_string()));
        assert!(context
            .hostnames
            .contains(&"web-01.ops.example.com".to_string()));
    }

    #[test]
    fn detects_sensitive_history_entries() {
        assert!(is_sensitive_command("export API_TOKEN=secret-value"));
    }
}
