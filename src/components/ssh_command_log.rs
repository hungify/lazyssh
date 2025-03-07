use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::action::Action;

pub struct ShhComandLog {
    command_log: Vec<String>,
}

impl ShhComandLog {
    pub fn new() -> Self {
        Self {
            command_log: Vec::new(),
        }
    }
}

impl Component for ShhComandLog {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::CaptureExecutedCommand(content) => {
                self.command_log.push(content);
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let command_log_text = self
            .command_log
            .iter()
            .map(|log| Line::from(log.as_str()))
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(command_log_text).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 100)))
                    .border_type(BorderType::Rounded)
                    .title("Command Log".fg(Color::White).bold())
                    .title_alignment(Alignment::Center),
            ),
            area,
        );
        Ok(())
    }
}
