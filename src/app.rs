use arboard::Clipboard;
use color_eyre::Result;
use dirs;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Stylize;
use ratatui::widgets::{
    Clear, List, ListItem, ListState, Padding, Scrollbar, ScrollbarOrientation, ScrollbarState,
    Wrap,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Paragraph},
    DefaultTerminal, Frame,
};
use std::collections::HashSet;
use std::fs;
use std::fs::read_to_string;
use std::iter::FromIterator;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use trash::delete;

use crate::event::{EventHandler, TerminalEvent};

const FORM_FIELD_COUNT: usize = 6;

struct KeyBindingItem {
    keycode: char,
    text: &'static str,
}

impl KeyBindingItem {
    fn new(keycode: char, text: &'static str) -> Self {
        Self { keycode, text }
    }
}

struct KeyBindings {
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

pub struct App {
    running: bool,
    command_log: Vec<String>,

    event_handler: EventHandler,

    ssh_files: Vec<String>,
    ssh_files_state: ListState,

    show_key_bindings: bool,
    show_confirm_delete: bool,
    show_create_form: bool,

    create_form_state: ListState,
    key_name: String,
    key_type: String,
    key_bits: String,
    passphrase: String,
    re_passphrase: String,
    key_types: Vec<&'static str>,
    selected_key_type_index: usize,
    bits_options: Vec<&'static str>,
    selected_bits_index: usize,
    comment: String,

    key_bindings: KeyBindings,
}

impl App {
    pub fn new(event_handler: EventHandler) -> Self {
        let mut ssh_files_state = ListState::default();
        ssh_files_state.select(Some(0));
        let mut create_form_state = ListState::default();
        create_form_state.select(Some(0));
        Self {
            running: true,

            ssh_files: Vec::new(),
            ssh_files_state,

            event_handler,

            show_confirm_delete: false,
            command_log: Vec::new(),

            show_key_bindings: false,
            key_bindings: KeyBindings::from_iter([
                ('n', "Create a SSH key"),
                ('a', "Add a SSH key to agent"),
                ('d', "Delete a SSH key"),
                ('c', "Copy a SSH public key to clipboard"),
                ('r', "Remove a SSH key from agent"),
            ]),

            show_create_form: false,
            key_name: String::new(),
            key_type: String::new(),
            key_bits: String::new(),
            passphrase: String::new(),
            re_passphrase: String::new(),
            key_types: vec!["rsa", "dsa", "ecdsa", "ed25519"],
            selected_key_type_index: 0,
            bits_options: vec!["1024", "2048", "4096"],
            selected_bits_index: 1,
            comment: String::new(),
            create_form_state,
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        self.ssh_files = self.load_ssh_files();
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            let event = self.event_handler.next()?;
            match event {
                TerminalEvent::Tick => {}
                TerminalEvent::Key(key_event) => {
                    self.on_key_event(key_event);
                }
                TerminalEvent::Mouse(_) => {}
                TerminalEvent::Resize(_, _) => {}
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area().inner(Margin {
            vertical: 0,
            horizontal: 0,
        });

        let main_chunks = self.create_main_layout(area);
        let content_chunks = self.create_content_layout(main_chunks[0]);
        let right_chunks = self.create_right_layout(content_chunks[1]);

        self.render_ssh_files(frame, content_chunks[0]);
        self.render_ssh_content(frame, right_chunks[0]);
        self.render_ssh_agent_status(frame, right_chunks[1]);
        self.render_command_log(frame, right_chunks[2]);
        self.render_footer(frame, main_chunks[1]);

        if self.show_key_bindings {
            self.render_key_bindings_popup(frame);
        }

        if self.show_confirm_delete {
            self.render_confirm_delete_popup(frame);
        }

        if self.show_create_form {
            self.render_create_form(frame);
        }
    }

    fn create_main_layout(&self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(93), Constraint::Percentage(7)].as_ref())
            .split(area)
            .to_vec()
    }

    fn create_content_layout(&self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
            .split(area)
            .to_vec()
    }

    fn create_right_layout(&self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60),
                Constraint::Percentage(7),
                Constraint::Percentage(33),
            ])
            .split(area)
            .to_vec()
    }

    fn truncate_with_ellipsis(&self, text: &str, max_width: usize) -> String {
        let visible_end_length = max_width - 10;
        if text.len() <= visible_end_length {
            return text.to_string();
        }

        let half_width = (visible_end_length) / 2;
        let remainder = (visible_end_length) % 2;

        let start = &text[..half_width];
        let end = &text[text.len() - (half_width + remainder)..];

        format!("{}...{}", start, end)
    }

    fn render_ssh_files(&self, frame: &mut Frame, area: Rect) {
        let available_width = area.width as usize;

        let items: Vec<ListItem> = self
            .ssh_files
            .iter()
            .map(|file| {
                let ellipsis_file = self.truncate_with_ellipsis(file, available_width);

                let style = if file.ends_with(".pub") {
                    Style::default()
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                ListItem::new(ellipsis_file.to_string()).style(style)
            })
            .collect();

        let current_selection_info = format!(
            "|{} of {}|",
            self.ssh_files_state.selected().unwrap_or(0) + 1,
            self.ssh_files.len()
        );

        let list = List::new(items)
            .block(
                Block::bordered()
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 100)))
                    .title(
                        "SSH Files"
                            .fg(Color::Reset)
                            .bold()
                            .underlined()
                            .into_centered_line(),
                    )
                    .title_bottom(Line::from(current_selection_info).alignment(Alignment::Center)),
            )
            .highlight_style(Style::default().fg(Color::Magenta).slow_blink())
            .highlight_symbol("➤ ");

        frame.render_stateful_widget(list, area, &mut self.ssh_files_state.clone());

        self.render_scrollbar(
            frame,
            area,
            self.ssh_files.len(),
            self.ssh_files_state.selected().unwrap_or_default(),
        );
    }

    fn render_scrollbar(
        &self,
        frame: &mut Frame,
        area: Rect,
        total_items: usize,
        selected_index: usize,
    ) {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::new(total_items).position(selected_index);
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    fn render_ssh_content(&self, frame: &mut Frame, area: Rect) {
        let ssh_content = self.load_ssh_content();
        frame.render_widget(
            Paragraph::new(ssh_content).wrap(Wrap { trim: true }).block(
                Block::default()
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 100)))
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("SSH Content".fg(Color::White).bold())
                    .title_alignment(Alignment::Center),
            ),
            area,
        );
    }

    fn render_ssh_agent_status(&self, frame: &mut Frame, area: Rect) {
        let agent_status = self.check_ssh_agent_status();
        frame.render_widget(
            Paragraph::new(agent_status).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 100)))
                    .border_type(BorderType::Rounded)
                    .title("SSH Agent Status".fg(Color::White).bold())
                    .title_alignment(Alignment::Center),
            ),
            area,
        );
    }

    fn render_command_log(&self, frame: &mut Frame, area: Rect) {
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
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
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
    }

    fn render_key_bindings_popup(&mut self, frame: &mut Frame) {
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
            popup_area.y + popup_area.height / 6,
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

        frame.render_stateful_widget(list, popup_rect, &mut self.key_bindings.state);
    }

    fn render_confirm_delete_popup(&self, frame: &mut Frame) {
        let title = Block::default()
            .title("Confirm Delete")
            .borders(ratatui::widgets::Borders::ALL)
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
    }

    fn load_ssh_files(&self) -> Vec<String> {
        let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
        if ssh_dir.exists() {
            let mut private_keys = HashSet::new();
            let mut public_keys = HashSet::new();
            let mut other_files = Vec::new();
            if let Ok(entries) = fs::read_dir(ssh_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Some(file_name) = entry.path().file_name().and_then(|n| n.to_str()) {
                        if file_name.ends_with(".pub") {
                            public_keys.insert(file_name.trim_end_matches(".pub").to_string());
                        } else if !file_name.ends_with(".pub") {
                            private_keys.insert(file_name.to_string());
                        } else {
                            other_files.push(file_name.to_string());
                        }
                    }
                }
            }
            let mut ssh_files: Vec<String> = private_keys
                .intersection(&public_keys)
                .map(|key| format!("{} - {}.pub", key, key))
                .collect();
            ssh_files.extend(private_keys.difference(&public_keys).cloned());
            ssh_files.extend(
                public_keys
                    .difference(&private_keys)
                    .map(|key| format!("{}.pub", key)),
            );
            ssh_files.extend(other_files);
            ssh_files
        } else {
            vec!["No SSH files found".to_string()]
        }
    }

    fn load_ssh_content(&self) -> String {
        let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
        if let Some(selected_file) = self
            .ssh_files
            .get(self.ssh_files_state.selected().unwrap_or(0))
        {
            let file_name = selected_file.split(" - ").next().unwrap();
            let path = if selected_file.contains(" - ") {
                ssh_dir.join(format!("{}.pub", file_name))
            } else {
                ssh_dir.join(file_name)
            };

            read_to_string(path).unwrap_or_else(|_| "Failed to read file content".to_string())
        } else {
            "No file selected".to_string()
        }
    }

    fn check_ssh_agent_status(&self) -> String {
        if let Some(selected_file) = self
            .ssh_files
            .get(self.ssh_files_state.selected().unwrap_or(0))
        {
            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let path = ssh_dir.join(format!(
                "{}.pub",
                selected_file.split(" - ").next().unwrap()
            ));
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
        } else {
            "No file selected".to_string()
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

    fn toggle_keybindings(&mut self) {
        self.show_key_bindings = !self.show_key_bindings;
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        if self.show_confirm_delete {
            self.handle_confirm_delete_key_event(key);
            return;
        }

        if self.show_create_form {
            self.handle_create_form_key_event(key);
            return;
        }

        if self.show_key_bindings {
            self.handle_key_bindings_key_event(key);
            return;
        }

        self.handle_general_key_event(key);
    }

    fn handle_confirm_delete_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.confirm_delete_ssh_key();
                self.toggle_confirm_delete();
            }
            KeyCode::Esc => {
                self.toggle_confirm_delete();
            }
            _ => {}
        }
    }

    fn handle_create_form_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if self.passphrase == self.re_passphrase {
                    self.create_ssh_key();
                } else {
                    self.command_log
                        .push("Passphrases do not match".to_string());
                }
            }
            KeyCode::Esc => self.toggle_create_ssh_key(),
            KeyCode::Tab => self.select_next_form_field(),
            KeyCode::BackTab => self.select_previous_form_field(),
            KeyCode::Char(c) => self.handle_char_input(c),
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Delete => self.handle_delete(),
            KeyCode::Up => self.handle_up_key(),
            KeyCode::Down => self.handle_down_key(),
            _ => {}
        }
    }

    fn select_next_form_field(&mut self) {
        let next_index = (self.create_form_state.selected().unwrap_or(0) + 1) % FORM_FIELD_COUNT;
        self.create_form_state.select(Some(next_index));
    }

    fn select_previous_form_field(&mut self) {
        let prev_index = if self.create_form_state.selected().unwrap_or(0) == 0 {
            FORM_FIELD_COUNT - 1
        } else {
            self.create_form_state.selected().unwrap_or(0) - 1
        };
        self.create_form_state.select(Some(prev_index));
    }

    fn handle_char_input(&mut self, c: char) {
        match self.create_form_state.selected() {
            Some(0) => self.key_name.push(c),
            Some(3) => self.passphrase.push(c),
            Some(4) => self.re_passphrase.push(c),
            Some(5) => self.comment.push(c),
            _ => {}
        }
    }

    fn handle_backspace(&mut self) {
        match self.create_form_state.selected() {
            Some(0) => self.key_name.pop(),
            Some(3) => self.passphrase.pop(),
            Some(4) => self.re_passphrase.pop(),
            Some(5) => self.comment.pop(),
            _ => None,
        };
    }

    fn handle_delete(&mut self) {
        match self.create_form_state.selected() {
            Some(0) => self.key_name.clear(),
            Some(3) => self.passphrase.clear(),
            Some(4) => self.re_passphrase.clear(),
            Some(5) => self.comment.clear(),
            _ => {}
        };
    }

    fn handle_up_key(&mut self) {
        if let Some(1) = self.create_form_state.selected() {
            self.selected_key_type_index = if self.selected_key_type_index == 0 {
                self.key_types.len() - 1
            } else {
                self.selected_key_type_index - 1
            };
        } else if let Some(2) = self.create_form_state.selected() {
            self.selected_bits_index = if self.selected_bits_index == 0 {
                self.bits_options.len() - 1
            } else {
                self.selected_bits_index - 1
            };
        }
    }

    fn handle_down_key(&mut self) {
        if let Some(1) = self.create_form_state.selected() {
            self.selected_key_type_index =
                if self.selected_key_type_index == self.key_types.len() - 1 {
                    0
                } else {
                    self.selected_key_type_index + 1
                };
        } else if let Some(2) = self.create_form_state.selected() {
            self.selected_bits_index = if self.selected_bits_index == self.bits_options.len() - 1 {
                0
            } else {
                self.selected_bits_index + 1
            };
        }
    }

    fn handle_key_bindings_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.execute_selected_key_binding(),
            KeyCode::Up => self.select_previous_key_binding(),
            KeyCode::Down => self.select_next_key_binding(),
            KeyCode::Esc | KeyCode::Char('?') => self.toggle_keybindings(),
            _ => {}
        }
    }

    fn execute_selected_key_binding(&mut self) {
        if let Some(selected) = self.key_bindings.state.selected() {
            let key_binding = &self.key_bindings.items[selected];
            self.handle_general_key_event(KeyEvent::new(
                KeyCode::Char(key_binding.keycode),
                KeyModifiers::NONE,
            ));
            self.toggle_keybindings();
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

    fn select_next_ssh_file(&mut self) {
        let i = match self.ssh_files_state.selected() {
            Some(i) => {
                if i >= self.ssh_files.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.ssh_files_state.select(Some(i));
    }

    fn select_previous_ssh_file(&mut self) {
        let i = match self.ssh_files_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.ssh_files_state.select(Some(i));
    }

    fn handle_general_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) => self.quit(),
            (_, KeyCode::Char('?')) => self.toggle_keybindings(),
            (_, KeyCode::Char('n')) => self.toggle_create_ssh_key(),
            (_, KeyCode::Char('a')) => self.add_ssh_key_to_agent(),
            (_, KeyCode::Char('d')) => self.toggle_confirm_delete(),
            (_, KeyCode::Char('c')) => self.copy_ssh_key_to_clipboard(),
            (_, KeyCode::Char('r')) => self.remove_ssh_key_from_agent(),
            (_, KeyCode::Down) => self.select_next_ssh_file(),
            (_, KeyCode::Up) => self.select_previous_ssh_file(),
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }

    fn toggle_create_ssh_key(&mut self) {
        self.show_create_form = !self.show_create_form;
        if self.show_create_form {
            self.create_form_state.select(Some(0));
        }
    }

    fn render_create_form(&self, frame: &mut Frame) {
        let input_chunks = self.create_form_layout(frame.area());

        let name_input = self.create_input_field("Name", &self.key_name, 0);
        let type_input = self.create_select_field(
            "Type (use arrow keys to change)",
            &self.key_types,
            self.selected_key_type_index,
            1,
        );
        let bits_input = self.create_select_field(
            "Bits (use arrow keys to change)",
            &self.bits_options,
            self.selected_bits_index,
            2,
        );
        let masked_passphrase = "*".repeat(self.passphrase.len());
        let masked_re_passphrase = "*".repeat(self.re_passphrase.len());

        let passphrase_input = self.create_input_field("Passphrase", &masked_passphrase, 3);
        let re_passphrase_input =
            self.create_input_field("Re-enter Passphrase", &masked_re_passphrase, 4);
        let comment_input = self.create_input_field("Comment", &self.comment, 5);

        frame.render_widget(Clear, input_chunks[0]);
        frame.render_widget(Clear, input_chunks[1]);
        frame.render_widget(Clear, input_chunks[2]);
        frame.render_widget(Clear, input_chunks[3]);
        frame.render_widget(Clear, input_chunks[4]);
        frame.render_widget(Clear, input_chunks[5]);

        frame.render_widget(name_input, input_chunks[0]);
        frame.render_widget(type_input, input_chunks[1]);
        frame.render_widget(bits_input, input_chunks[2]);
        frame.render_widget(passphrase_input, input_chunks[3]);
        frame.render_widget(re_passphrase_input, input_chunks[4]);
        frame.render_widget(comment_input, input_chunks[5]);
    }

    fn create_form_layout(&self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                (0..FORM_FIELD_COUNT)
                    .map(|_| Constraint::Length(3))
                    .collect::<Vec<_>>(),
            )
            .split(Rect::new(
                area.x + area.width / 4,
                area.y + area.height / 6,
                area.width / 2,
                area.height / 2,
            ))
            .to_vec()
    }

    fn create_input_field<'a>(&self, title: &str, value: &'a str, index: usize) -> Paragraph<'a> {
        let border_style = if self.create_form_state.selected() == Some(index) {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        Paragraph::new(value).block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .title(title.to_string())
                .title_style(border_style),
        )
    }

    fn create_select_field<'a>(
        &self,
        title: &str,
        options: &[&'a str],
        selected_index: usize,
        index: usize,
    ) -> Paragraph<'a> {
        let selected_option = options[selected_index];
        let border_style = if self.create_form_state.selected() == Some(index) {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        Paragraph::new(selected_option).block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .title(title.to_string())
                .title_style(border_style),
        )
    }

    fn create_ssh_key(&mut self) {
        let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
        let key_type = &self.key_types[self.selected_key_type_index];
        let key_bits = self.bits_options[self.selected_bits_index];
        let now = SystemTime::now();
        let current_time = now
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let key_name_with_fallback = if self.key_name.trim().is_empty() {
            "id_".to_string() + key_type + "_" + &current_time
        } else {
            self.key_name.trim().to_string()
        };

        let key_path = ssh_dir.join(&key_name_with_fallback);
        let key_path_str = key_path.display().to_string();

        let output = Command::new("ssh-keygen")
            .arg("-t")
            .arg(key_type)
            .arg("-b")
            .arg(key_bits)
            .arg("-f")
            .arg(&key_path)
            .arg("-N")
            .arg(&self.passphrase)
            .arg("-C")
            .arg(&self.comment)
            .output()
            .expect("Failed to execute ssh-keygen");

        let masked_passphrase = "*".repeat(self.passphrase.len());
        self.command_log.push(format!(
            "ssh-keygen -t {} -b {} -f {} -N {} -C {}",
            key_type, key_bits, key_path_str, masked_passphrase, self.comment
        ));
        if output.status.success() {
            self.ssh_files = self.load_ssh_files();
            self.ssh_files_state.select(Some(0));
            self.show_create_form = false;
            self.clear_input_fields();
            self.command_log
                .push(format!("SSH key created: {}", key_path.display()));
        } else {
            self.command_log.push(format!(
                "Failed to create SSH key: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    fn clear_input_fields(&mut self) {
        self.key_name.clear();
        self.key_type.clear();
        self.key_bits.clear();
        self.passphrase.clear();
        self.re_passphrase.clear();
        self.comment.clear();
    }

    fn add_ssh_key_to_agent(&mut self) {
        if let Some(selected_file) = self
            .ssh_files
            .get(self.ssh_files_state.selected().unwrap_or(0))
        {
            if !selected_file.contains(" - ") {
                self.command_log.push(format!(
                    "Cannot add: {} is not a private key file of an SSH pair",
                    selected_file
                ));
                return;
            }

            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let path = ssh_dir.join(selected_file.split(" - ").next().unwrap());

            match self.get_fingerprint(&path) {
                Ok(fingerprint) => {
                    if self.is_key_in_agent(&fingerprint) {
                        self.command_log.push(format!(
                            "ssh-add {} -> SSH key is already added to agent",
                            path.display()
                        ));
                        return;
                    }
                }
                Err(err) => {
                    self.command_log.push(err);
                    return;
                }
            }

            let output = Command::new("ssh-add")
                .arg(&path)
                .output()
                .expect("Failed to execute ssh-add");

            if output.status.success() {
                self.command_log.push(format!(
                    "ssh-add {} -> SSH key added to agent",
                    path.display()
                ));
            } else {
                self.command_log.push(format!(
                    "ssh-add {} -> Failed to add SSH key to agent: {}",
                    path.display(),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    }

    fn toggle_confirm_delete(&mut self) {
        self.show_confirm_delete = !self.show_confirm_delete;
    }

    fn confirm_delete_ssh_key(&mut self) {
        if let Some(selected_file) = self
            .ssh_files
            .get(self.ssh_files_state.selected().unwrap_or(0))
        {
            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let private_key_path = ssh_dir.join(selected_file.split(" - ").next().unwrap());
            let public_key_path = ssh_dir.join(format!("{}.pub", private_key_path.display()));

            let private_key_deleted = delete(&private_key_path).is_ok();
            let public_key_deleted = delete(&public_key_path).is_ok();

            if private_key_deleted || public_key_deleted {
                self.command_log.push(format!(
                    "Move to trash: {} -> SSH key moved to trash",
                    private_key_path.display()
                ));
                self.ssh_files
                    .remove(self.ssh_files_state.selected().unwrap_or(0));
                self.ssh_files_state.select(Some(
                    self.ssh_files_state
                        .selected()
                        .unwrap_or(0)
                        .saturating_sub(1),
                ));
            } else {
                let other_file_path = ssh_dir.join(selected_file);
                if delete(&other_file_path).is_ok() {
                    self.command_log.push(format!(
                        "Move to trash: {} -> SSH key moved to trash",
                        other_file_path.display()
                    ));
                    self.ssh_files
                        .remove(self.ssh_files_state.selected().unwrap_or(0));
                    self.ssh_files_state.select(Some(
                        self.ssh_files_state
                            .selected()
                            .unwrap_or(0)
                            .saturating_sub(1),
                    ));
                }
            }
        }
    }

    fn copy_ssh_key_to_clipboard(&mut self) {
        if let Some(selected_file) = self
            .ssh_files
            .get(self.ssh_files_state.selected().unwrap_or(0))
        {
            if !selected_file.contains(" - ") {
                self.command_log.push(format!(
                    "Cannot copy: {} is not a public key file of an SSH pair",
                    selected_file
                ));
                return;
            }

            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let path = ssh_dir.join(format!(
                "{}.pub",
                selected_file.split(" - ").next().unwrap()
            ));
            match read_to_string(&path) {
                Ok(content) => {
                    let mut clipboard = Clipboard::new().unwrap();
                    clipboard.set_text(content).unwrap();
                    self.command_log.push(format!(
                        "Copy to clipboard: {} -> SSH public key copied to clipboard",
                        path.display()
                    ));
                }
                Err(err) => {
                    self.command_log
                        .push(format!("Failed to copy SSH public key: {}", err));
                }
            }
        }
    }

    fn remove_ssh_key_from_agent(&mut self) {
        if let Some(selected_file) = self
            .ssh_files
            .get(self.ssh_files_state.selected().unwrap_or(0))
        {
            if !selected_file.contains(" - ") {
                self.command_log.push(format!(
                    "Cannot remove: {} is not a private key file of an SSH pair",
                    selected_file
                ));
                return;
            }

            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let path = ssh_dir.join(selected_file.split(" - ").next().unwrap());

            match self.get_fingerprint(&path) {
                Ok(fingerprint) => {
                    if !self.is_key_in_agent(&fingerprint) {
                        self.command_log.push(format!(
                            "ssh-add -d {} -> SSH key is not added to agent",
                            path.display()
                        ));
                        return;
                    }
                }
                Err(err) => {
                    self.command_log.push(err);
                    return;
                }
            }

            let output = Command::new("ssh-add")
                .arg("-d")
                .arg(&path)
                .output()
                .expect("Failed to execute ssh-add");

            if output.status.success() {
                self.command_log.push(format!(
                    "ssh-add -d {} -> SSH key removed from agent",
                    path.display()
                ));
            } else {
                self.command_log.push(format!(
                    "ssh-add -d {} -> Failed to remove SSH key from agent: {}",
                    path.display(),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    }
}
