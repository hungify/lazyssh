use super::Component;
use crate::action::Action;
use color_eyre::{eyre::Ok, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
    Frame,
};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

struct KeyBindingItem {
    keycode: char,
    text: &'static str,
}

impl KeyBindingItem {
    fn new(keycode: char, text: &'static str) -> Self {
        Self { keycode, text }
    }
}
pub struct KeyBindings {
    items: Vec<KeyBindingItem>,
    state: ListState,
}

impl FromIterator<(char, &'static str)> for KeyBindings {
    fn from_iter<I: IntoIterator<Item = (char, &'static str)>>(iter: I) -> Self {
        let items: Vec<KeyBindingItem> = iter
            .into_iter()
            .map(|(keycode, text)| KeyBindingItem::new(keycode, text))
            .collect();
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }
}

pub struct KeyBindingModal {
    key_bindings: KeyBindings,
    show_key_bindings: bool,
    command_tx: Option<UnboundedSender<Action>>,
}

impl KeyBindingModal {
    pub fn new() -> Self {
        Self {
            key_bindings: KeyBindings::from_iter([
                ('n', "Create a SSH key"),
                ('a', "Add a SSH key to agent"),
                ('d', "Delete a SSH key"),
                ('c', "Copy a SSH public key to clipboard"),
                ('r', "Remove a SSH key from agent"),
            ]),
            show_key_bindings: false,
            command_tx: None,
        }
    }

    fn toggle_keybindings(&mut self) {
        self.show_key_bindings = !self.show_key_bindings;
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(Action::ToggleKeyBindings(self.show_key_bindings));
        }
        if self.show_key_bindings {
            self.key_bindings.state.select(Some(0));
        }
    }

    fn select_previous_key_binding(&mut self) {
        let i = match self.key_bindings.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.key_bindings.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.key_bindings.state.select(Some(i));
    }

    fn select_next_key_binding(&mut self) {
        let i = match self.key_bindings.state.selected() {
            Some(i) => {
                if i == self.key_bindings.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.key_bindings.state.select(Some(i));
    }

    fn execute_selected_key_binding(&mut self) {
        if let Some(selected) = self.key_bindings.state.selected() {
            let key_binding = &self.key_bindings.items[selected];
            if let Some(tx) = &self.command_tx {
                let _ = tx.send(Action::ExecuteSelectedKeyBinding(KeyEvent::new(
                    KeyCode::Char(key_binding.keycode),
                    KeyModifiers::NONE,
                )));
            }
        }
    }
}

impl Component for KeyBindingModal {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::ClearScreen => {}
            Action::ToggleKeyBindings(is_open) => {
                self.show_key_bindings = is_open;
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _: Rect) -> Result<()> {
        if !self.show_key_bindings {
            return Ok(());
        }

        let title = Block::default()
            .title("Key Bindings")
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(BorderType::Rounded)
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(Color::Green))
            .padding(Padding {
                bottom: 0,
                left: 1,
                right: 1,
                top: 1,
            });

        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(30)].as_ref())
            .split(frame.area())[1];

        let popup_rect = Rect::new(
            popup_area.x + popup_area.width / 3,
            popup_area.y / 2,
            popup_area.width / 3,
            popup_area.height,
        );

        let items: Vec<ListItem> = self
            .key_bindings
            .items
            .iter()
            .map(|item| ListItem::from(format!("{} {}", item.keycode, item.text)))
            .collect();

        let list = List::new(items).block(title).highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(Clear, popup_rect);
        frame.render_stateful_widget(list, popup_rect, &mut self.key_bindings.state);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                self.toggle_keybindings();
                Ok(None)
            }

            KeyCode::Enter => {
                self.execute_selected_key_binding();
                Ok(None)
            }
            KeyCode::Up => {
                self.select_previous_key_binding();
                Ok(None)
            }
            KeyCode::Down => {
                self.select_next_key_binding();
                Ok(None)
            }

            _ => Ok(None),
        }
    }
}
