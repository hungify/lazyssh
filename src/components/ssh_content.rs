use std::fs;

use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::action::Action;

pub struct SshContent {
    content: String,
}

impl SshContent {
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn load_ssh_content(&mut self, file_name: &str) {
        let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
        let path = ssh_dir.join(file_name);
        let file_content =
            fs::read_to_string(path).unwrap_or_else(|_| "Failed to read file content".to_string());
        if file_content.is_empty() {
            self.content = "File is empty".to_string();
        } else {
            self.content = file_content;
        }
    }
}

impl Component for SshContent {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::SelectedSshFileContent(filename) => {
                self.load_ssh_content(&filename);
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(
            Paragraph::new(self.content.clone())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .border_style(Style::default().fg(Color::Rgb(100, 100, 100)))
                        .borders(ratatui::widgets::Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title("SSH Content".fg(Color::White).bold())
                        .title_alignment(Alignment::Center),
                ),
            area,
        );
        Ok(())
    }
}
