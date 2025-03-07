use super::Component;
use crate::action::Action;
use color_eyre::{eyre::Ok, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
    Frame,
};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

pub struct ConfirmDeleteModal {
    show_confirm_delete: bool,
    command_tx: Option<UnboundedSender<Action>>,
}

impl ConfirmDeleteModal {
    pub fn new() -> Self {
        Self {
            show_confirm_delete: false,
            command_tx: None,
        }
    }

    fn toggle_confirm_delete(&mut self) {
        self.show_confirm_delete = !self.show_confirm_delete;
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(Action::ToggleConfirmDelete(self.show_confirm_delete));
        }
    }

    fn handle_delete(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(Action::DeleteSelectedFile);
        }
    }
}

impl Component for ConfirmDeleteModal {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::ClearScreen => {}
            Action::ToggleConfirmDelete(is_open) => {
                self.show_confirm_delete = is_open;
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _: Rect) -> Result<()> {
        if !self.show_confirm_delete {
            return Ok(());
        }

        let title = Block::default()
            .title("Confirm Delete")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Red));

        let popup = Paragraph::new(vec![
            Line::from("Are you sure you want to delete this SSH key?"),
            Line::from("Note: You can recover the key from the trash."),
        ])
        .block(title)
        .alignment(Alignment::Left);

        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(30)].as_ref())
            .split(frame.area())[1];

        let popup_area = Rect::new(
            popup_area.x + popup_area.width / 3,
            popup_area.y + popup_area.height / 4,
            popup_area.width / 3,
            popup_area.height / 3,
        );

        frame.render_widget(Clear, popup_area);
        frame.render_widget(popup, popup_area);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if !self.show_confirm_delete {
            return Ok(None);
        }

        match key.code {
            KeyCode::Esc => {
                self.toggle_confirm_delete();
                Ok(None)
            }

            KeyCode::Enter => {
                self.handle_delete();
                self.toggle_confirm_delete();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}
