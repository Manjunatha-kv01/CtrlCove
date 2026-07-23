use crate::operations;
use serde::Serialize;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

pub const MAX_HISTORY_COMMANDS: usize = 1_000;
const MAX_COMMAND_CHARS: usize = 4_000;

#[derive(Debug, Serialize)]
pub struct TerminalHistoryImport {
    pub shell: String,
    pub commands: Vec<String>,
    pub skipped_sensitive: usize,
    pub skipped_irrelevant: usize,
}

pub fn read_history(shell: &str) -> Result<TerminalHistoryImport, String> {
    let path = history_path(shell)?;
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("Unable to read {} history: {error}", shell.to_lowercase()))?;
    let mut commands = Vec::new();
    let mut seen = HashSet::new();
    let mut skipped_sensitive = 0;
    let mut skipped_irrelevant = 0;

    for line in contents.lines().rev() {
        let command = normalize_history_line(line, shell);
        if command.is_empty() || command.chars().count() > MAX_COMMAND_CHARS {
            skipped_irrelevant += 1;
            continue;
        }
        if operations::is_sensitive_command(&command) {
            skipped_sensitive += 1;
            continue;
        }
        if operations::is_low_signal_command(&command)
            || !operations::analyze(&command, Some(shell)).is_operational()
        {
            skipped_irrelevant += 1;
            continue;
        }
        if seen.insert(command.clone()) {
            commands.push(command);
        }
        if commands.len() == MAX_HISTORY_COMMANDS {
            break;
        }
    }

    commands.reverse();
    Ok(TerminalHistoryImport {
        shell: shell.to_string(),
        commands,
        skipped_sensitive,
        skipped_irrelevant,
    })
}

pub fn newest_commands(commands: &[String], limit: usize) -> Vec<String> {
    let start = commands.len().saturating_sub(limit);
    commands[start..].to_vec()
}

fn history_path(shell: &str) -> Result<PathBuf, String> {
    let home = env::var_os("HOME")
        .ok_or_else(|| "Unable to locate the current user home directory.".to_string())?;
    match shell {
        "Bash" => Ok(PathBuf::from(home).join(".bash_history")),
        "Zsh" => Ok(PathBuf::from(home).join(".zsh_history")),
        _ => Err("Unsupported terminal history source.".to_string()),
    }
}

fn normalize_history_line(line: &str, shell: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return String::new();
    }

    if shell == "Zsh" && trimmed.starts_with(':') {
        return trimmed
            .split_once(';')
            .map(|(_, command)| command.trim().to_string())
            .unwrap_or_default();
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::{newest_commands, normalize_history_line};

    #[test]
    fn parses_zsh_extended_history() {
        assert_eq!(
            normalize_history_line(": 1700000000:0;systemctl status nginx", "Zsh"),
            "systemctl status nginx"
        );
    }

    #[test]
    fn keeps_the_newest_history_window_in_chronological_order() {
        let commands = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        assert_eq!(newest_commands(&commands, 2), vec!["two", "three"]);
    }
}
