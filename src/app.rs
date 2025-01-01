use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use dirs;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Positions, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{block::Position, Block, BorderType, Padding, Paragraph},
    DefaultTerminal, Frame,
};
use std::collections::HashSet;
use std::fs;
use std::fs::read_to_string;
use std::process::Command;

#[derive(Debug, Default)]
pub struct App {
    running: bool,
    selected_index: usize,
    ssh_files: Vec<String>,
    show_help: bool,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        self.ssh_files = self.load_ssh_files();
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_crossterm_events()?;
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

        self.render_ssh_files(frame, content_chunks[0]);
        self.render_ssh_content(frame, content_chunks[1]);
        self.render_footer(frame, main_chunks[1]);

        if self.show_help {
            self.render_help_popup(frame);
        }
    }

    fn create_main_layout(&self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
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

    fn render_ssh_files(&self, frame: &mut Frame, area: Rect) {
        let ssh_text = self
            .ssh_files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let prefix = if file.contains(" - ") {
                    format!("{}: ", i + 1)
                } else {
                    "".to_string()
                };
                if i == self.selected_index {
                    Line::from(format!("{}{}", prefix, file)).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Line::from(format!("{}{}", prefix, file))
                }
            })
            .collect::<Vec<_>>();

        let current_selection_info =
            format!("{} of {}", self.selected_index + 1, self.ssh_files.len());

        frame.render_widget(
            Paragraph::new(ssh_text).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("SSH Files")
                    .title_alignment(Alignment::Left)
                    .title_bottom(Line::from(current_selection_info).alignment(Alignment::Right)),
            ),
            area,
        );
    }

    fn render_ssh_content(&self, frame: &mut Frame, area: Rect) {
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
            .split(area);

        let ssh_content = self.load_ssh_content();
        frame.render_widget(
            Paragraph::new(ssh_content).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("SSH Content")
                    .title_alignment(Alignment::Center),
            ),
            right_chunks[0],
        );

        let agent_status = self.check_ssh_agent_status();
        frame.render_widget(
            Paragraph::new(agent_status).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("SSH Agent Status")
                    .title_alignment(Alignment::Center),
            ),
            right_chunks[1],
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let footer_text = if self.show_help {
            "Press `Shift + ?` to close help."
        } else {
            "Press `Esc`, `Ctrl-C` or `q` to quit. Use arrow keys to navigate. | Keybindings: ?"
        };
        frame.render_widget(
            Paragraph::new(footer_text).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Information")
                    .title_alignment(Alignment::Center),
            ),
            area,
        );
    }

    fn render_help_popup(&self, frame: &mut Frame) {
        let help_text = "Keybindings:\n\n\
                         Arrow Up/Down: Navigate\n\
                         Esc, Ctrl-C, q: Quit\n\
                         Shift + ?: Show this help";
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(frame.area())[1];

        frame.render_widget(
            Paragraph::new(help_text).block(
                Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Help"),
            ),
            popup_area,
        );
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
        if let Some(selected_file) = self.ssh_files.get(self.selected_index) {
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
        if let Some(selected_file) = self.ssh_files.get(self.selected_index) {
            let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
            let path = ssh_dir.join(format!(
                "{}.pub",
                selected_file.split(" - ").next().unwrap()
            ));
            if let Ok(file_content) = read_to_string(path) {
                let output = Command::new("ssh-add")
                    .arg("-L")
                    .output()
                    .expect("Failed to execute ssh-add");

                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains(&file_content) {
                    "SSH key is added to agent".to_string()
                } else {
                    "SSH key is not added to agent".to_string()
                }
            } else {
                "Failed to read file content".to_string()
            }
        } else {
            "No file selected".to_string()
        }
    }

    fn handle_crossterm_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => self.on_key_event(key),
                Event::Mouse(event) => {
                    self.on_mouse_event(event);
                    println!("Selected index after click: {}", self.selected_index);
                }
                Event::Resize(_, _) => {}
            }
        }
        Ok(())
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Down) => {
                if self.selected_index < self.ssh_files.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            (_, KeyCode::Up) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            _ => {}
        }
    }

    fn on_mouse_event(&mut self, event: MouseEvent) {
        if let MouseEventKind::Down(_) = event.kind {
            let list_start = 1;
            let list_end = list_start + self.ssh_files.len() as u16;

            if event.column < 50 && event.row >= list_start && event.row < list_end {
                self.selected_index = (event.row - list_start) as usize;
            }
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
