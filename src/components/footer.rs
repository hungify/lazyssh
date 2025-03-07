use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Footer {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    show_key_bindings: bool,
}

impl Footer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for Footer {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::Render => {}
            Action::ToggleKeyBindings(is_open) => {
                self.show_key_bindings = is_open;
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let footer_text = if self.show_key_bindings {
            "Use ↓↑ to move | Execute: <enter> | Keybindings: ? | Close: <esc>"
        } else {
            "Use ↓↑ to move | Create: n | Delete: d | Add to agent: a | Remove from agent: r | Copy to clipboard: c | Keybindings: ? | Quit: q"
        };
        frame.render_widget(
            Paragraph::new(footer_text).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 100)))
                    .border_type(BorderType::Rounded)
                    .title("Information".fg(Color::White).bold())
                    .title_alignment(Alignment::Center),
            ),
            area,
        );
        Ok(())
    }
}
