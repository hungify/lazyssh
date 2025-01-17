use lazyssh::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let event_handler = event::EventHandler::new();
    let result = app::App::new(event_handler).run(terminal);
    ratatui::restore();
    result
}
