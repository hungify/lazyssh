use arboard::Clipboard;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Margin, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, List, ListItem, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::{collections::HashSet, fs, process::Command};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::action::Action;

#[derive(Default)]
pub struct SshFiles {
    command_tx: Option<UnboundedSender<Action>>,
    files_name: Vec<String>,
    files_state: ListState,
    show_key_bindings: bool,
    show_confirm_delete: bool,
}

impl SshFiles {
    pub fn new() -> Self {
        let mut ssh_files_state = ListState::default();
        ssh_files_state.select(Some(0));
        let ssh_files = Self::load_ssh_files();

        Self {
            files_name: ssh_files,
            files_state: ssh_files_state,
            command_tx: None,
            show_key_bindings: false,
            show_confirm_delete: false,
        }
    }

    fn load_ssh_files() -> Vec<String> {
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
                .map(|key| format!("{}.pub", key))
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

    fn truncate_with_ellipsis(&mut self, text: &str, max_width: usize) -> String {
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

    fn select_next_ssh_file(&mut self) {
        let i = match self.files_state.selected() {
            Some(i) => {
                if i >= self.files_name.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.files_state.select(Some(i));
        self.dispatch_selected_file();
    }

    fn select_previous_ssh_file(&mut self) {
        let i = match self.files_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.files_state.select(Some(i));
        self.dispatch_selected_file();
    }

    fn dispatch_selected_file(&self) {
        if let Some(selected_file) = self
            .files_state
            .selected()
            .and_then(|i| self.files_name.get(i))
        {
            if let Some(tx) = &self.command_tx {
                let _ = tx.send(Action::SelectedSshFileContent(selected_file.to_string()));
            }
        }
    }

    fn dispatch_command_log(&self, command_log: String) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(Action::CaptureExecutedCommand(command_log));
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

    fn copy_ssh_key_to_clipboard(&mut self) {
        let mut command_trace_log = String::new();
        if let Some(selected_file) = self
            .files_state
            .selected()
            .and_then(|i| self.files_name.get(i))
        {
            if !selected_file.ends_with(".pub") {
                command_trace_log = format!(
                    "Cannot copy: {} is not a public key file of an SSH pair",
                    selected_file
                );
            } else {
                let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
                let path = ssh_dir.join(selected_file);
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        let mut clipboard = Clipboard::new().unwrap();
                        clipboard.set_text(content).unwrap();
                        command_trace_log = format!(
                            "Copy to clipboard: {} -> SSH public key copied to clipboard",
                            path.display()
                        );
                    }
                    Err(err) => {
                        command_trace_log = format!("Failed to copy SSH public key: {}", err);
                    }
                }
            }
        }
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(Action::CaptureExecutedCommand(command_trace_log));
        }
    }

    fn toggle_key_bindings(&mut self) {
        self.show_key_bindings = !self.show_key_bindings;
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(Action::ToggleKeyBindings(self.show_key_bindings));
        }
    }

    fn toggle_confirm_delete(&mut self) {
        self.show_confirm_delete = !self.show_confirm_delete;
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(Action::ToggleConfirmDelete(self.show_confirm_delete));
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

    fn add_ssh_key_to_agent(&mut self) {
        if let Some(selected_file) = self
            .files_state
            .selected()
            .and_then(|i| self.files_name.get(i))
        {
            if !selected_file.ends_with(".pub") {
                let command_log = format!(
                    "Cannot add: {} is not a public key file of an SSH pair",
                    selected_file
                );
                self.dispatch_command_log(command_log);
                return;
            }

            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let path = ssh_dir.join(selected_file.trim_end_matches(".pub"));
            match self.get_fingerprint(&path) {
                Ok(fingerprint) => {
                    if self.is_key_in_agent(&fingerprint) {
                        self.dispatch_command_log(format!(
                            "ssh-add {} -> SSH key is already added to agent",
                            path.display()
                        ));
                        return;
                    }
                }
                Err(err) => {
                    self.dispatch_command_log(format!("Failed to add SSH key to agent: {}", err));
                    return;
                }
            }

            let output = Command::new("ssh-add")
                .arg(&path)
                .output()
                .expect("Failed to execute ssh-add");

            if output.status.success() {
                self.dispatch_command_log(format!(
                    "ssh-add {} -> SSH key added to agent",
                    path.display()
                ));
                return;
            }

            self.dispatch_command_log(format!(
                "ssh-add {} -> Failed to add SSH key to agent: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    fn remove_ssh_key_from_agent(&mut self) {
        if let Some(selected_file) = self
            .files_state
            .selected()
            .and_then(|i| self.files_name.get(i))
        {
            if !selected_file.ends_with(".pub") {
                self.dispatch_command_log(format!(
                    "Cannot remove: {} is not a private key file of an SSH pair",
                    selected_file
                ));
                return;
            }

            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let path = ssh_dir.join(selected_file.trim_end_matches(".pub"));

            match self.get_fingerprint(&path) {
                Ok(fingerprint) => {
                    if !self.is_key_in_agent(&fingerprint) {
                        self.dispatch_command_log(format!(
                            "ssh-add -d {} -> SSH key is not added to agent",
                            path.display()
                        ));
                        return;
                    }
                }
                Err(err) => {
                    self.dispatch_command_log(format!(
                        "Failed to remove SSH key from agent: {}",
                        err
                    ));
                    return;
                }
            }

            let output = Command::new("ssh-add")
                .arg("-d")
                .arg(&path)
                .output()
                .expect("Failed to execute ssh-add");
            if output.status.success() {
                self.dispatch_command_log(format!(
                    "ssh-add -d {} -> SSH key removed from agent",
                    path.display()
                ));
                return;
            }

            self.dispatch_command_log(format!(
                "ssh-add -d {} -> Failed to remove SSH key from agent: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    fn handle_delete_ssh_file(&mut self) {
        if let Some(selected_file) = self
            .files_state
            .selected()
            .and_then(|i| self.files_name.get(i))
        {
            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let private_key_path = ssh_dir.join(selected_file.trim_end_matches(".pub"));
            let public_key_path = ssh_dir.join(selected_file);

            let private_key_deleted = trash::delete(&private_key_path).is_ok();
            let public_key_deleted = trash::delete(&public_key_path).is_ok();

            if private_key_deleted || public_key_deleted {
                self.dispatch_command_log(format!(
                    "Move to trash: {} -> SSH key moved to trash",
                    private_key_path.display()
                ));
                self.files_name
                    .remove(self.files_state.selected().unwrap_or(0));
                self.select_next_ssh_file();
            } else {
                let other_file_path = ssh_dir.join(selected_file);
                if trash::delete(&other_file_path).is_ok() {
                    self.dispatch_command_log(format!(
                        "Move to trash: {} -> SSH key moved to trash",
                        other_file_path.display()
                    ));
                    self.files_name
                        .remove(self.files_state.selected().unwrap_or(0));
                    self.files_state.select(Some(
                        self.files_state.selected().unwrap_or(0).saturating_sub(1),
                    ));
                }
            }
        }
    }
}

impl Component for SshFiles {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        self.dispatch_selected_file();
        Ok(())
    }
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let available_width = area.width as usize;

        let items: Vec<ListItem> = self
            .files_name
            .clone()
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
            self.files_state.selected().unwrap_or(0) + 1,
            self.files_name.len()
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

        frame.render_stateful_widget(list, area, &mut self.files_state.clone());
        let total_items = self.files_name.len();
        let selected_index = self.files_state.selected().unwrap_or(0);

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
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::Render => {}
            Action::ToggleKeyBindings(is_show) => {
                self.show_key_bindings = is_show;
            }
            Action::ExecuteSelectedKeyBinding(key) => {
                self.handle_key_event(key)?;
            }
            Action::DeleteSelectedFile => {
                self.handle_delete_ssh_file();
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.show_key_bindings && (key.code == KeyCode::Down || key.code == KeyCode::Up) {
            return Ok(None);
        }

        match key.code {
            KeyCode::Down => {
                self.select_next_ssh_file();
                Ok(None)
            }
            KeyCode::Up => {
                self.select_previous_ssh_file();
                Ok(None)
            }
            KeyCode::Char('c') => {
                self.copy_ssh_key_to_clipboard();
                self.toggle_key_bindings();
                Ok(None)
            }
            KeyCode::Char('?') => {
                self.toggle_key_bindings();
                Ok(None)
            }
            // KeyCode::Char('n') => self.toggle_create_ssh_key(),
            KeyCode::Char('a') => {
                self.add_ssh_key_to_agent();
                self.toggle_key_bindings();
                Ok(None)
            }
            KeyCode::Char('d') => {
                self.toggle_confirm_delete();
                if self.show_key_bindings {
                    self.toggle_key_bindings();
                }
                Ok(None)
            }
            KeyCode::Char('r') => {
                self.remove_ssh_key_from_agent();
                self.toggle_key_bindings();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}
