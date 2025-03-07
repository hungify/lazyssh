use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::action::Action;
use std::process::Command;

pub struct SshAgentStatus {
    agent_status: String,
}

impl SshAgentStatus {
    pub fn new() -> Self {
        Self {
            agent_status: "No file selected".to_string(),
        }
    }

    fn check_ssh_agent_status(&self, filename: String) -> String {
        let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
        let path = ssh_dir.join(filename);
        if path.exists() {
            match self.get_fingerprint(&path) {
                Ok(fingerprint) => {
                    if self.is_key_in_agent(&fingerprint) {
                        "SSH key is added to agent".to_string()
                    } else {
                        "SSH key is not added to agent".to_string()
                    }
                }
                Err(err) => err,
            }
        } else {
            "It's not a ssh key".to_string()
        }
    }

    fn get_fingerprint(&self, path: &std::path::Path) -> Result<String, String> {
        let output = Command::new("ssh-keygen")
            .arg("-lf")
            .arg(path)
            .output()
            .expect("Failed to execute ssh-keygen");

        if output.status.success() {
            let fingerprint = String::from_utf8_lossy(&output.stdout);
            Ok(fingerprint
                .split_whitespace()
                .nth(1)
                .unwrap_or("")
                .to_string())
        } else {
            Err("Failed to get SSH key fingerprint".to_string())
        }
    }

    fn is_key_in_agent(&self, fingerprint: &str) -> bool {
        let output = Command::new("ssh-add")
            .arg("-l")
            .output()
            .expect("Failed to execute ssh-add");

        if output.status.success() {
            let agent_keys = String::from_utf8_lossy(&output.stdout);
            agent_keys.lines().any(|line| line.contains(fingerprint))
        } else {
            false
        }
    }
}

impl Component for SshAgentStatus {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::SelectedSshFileContent(filename) => {
                let status = self.check_ssh_agent_status(filename);
                self.agent_status = status;
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(
            Paragraph::new(self.agent_status.clone()).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 100)))
                    .border_type(BorderType::Rounded)
                    .title("SSH Agent Status".fg(Color::White).bold())
                    .title_alignment(Alignment::Center),
            ),
            area,
        );
        Ok(())
    }
}
