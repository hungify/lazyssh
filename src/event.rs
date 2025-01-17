use color_eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventKind, MouseEvent};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Terminal events.
#[derive(Clone, Debug, PartialEq)]
pub enum TerminalEvent {
    /// Terminal tick.
    Tick,
    /// Key press.
    Key(KeyEvent),
    /// Mouse click/scroll.
    Mouse(MouseEvent),
    /// Terminal resize.
    Resize(u16, u16),
}

/// Terminal event handler.
#[allow(dead_code)]
#[derive(Debug)]
pub struct EventHandler {
    /// Tick rate.
    pub tick_rate: Duration,
    /// Event sender channel.
    pub sender: mpsc::Sender<TerminalEvent>,
    /// Event receiver channel.
    receiver: mpsc::Receiver<TerminalEvent>,
    /// Event handler thread.
    handler: thread::JoinHandle<()>,
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`].
    pub fn new() -> Self {
        let tick_rate = Duration::from_millis(5000);
        let (sender, receiver) = mpsc::channel();
        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);
                    if event::poll(timeout).expect("failed to poll new events") {
                        match event::read().expect("unable to read event") {
                            Event::Key(e) => {
                                if e.kind == KeyEventKind::Press {
                                    sender.send(TerminalEvent::Key(e))
                                } else {
                                    Ok(())
                                }
                            }
                            Event::Mouse(e) => sender.send(TerminalEvent::Mouse(e)),
                            Event::Resize(w, h) => sender.send(TerminalEvent::Resize(w, h)),
                            Event::FocusGained => Ok(()),
                            Event::FocusLost => Ok(()),
                            Event::Paste(_) => unimplemented!(),
                        }
                        .expect("failed to send terminal event")
                    }

                    if last_tick.elapsed() >= tick_rate {
                        sender
                            .send(TerminalEvent::Tick)
                            .expect("failed to send tick event");
                        last_tick = Instant::now();
                    }
                }
            })
        };
        Self {
            tick_rate,
            sender,
            receiver,
            handler,
        }
    }

    /// Receive the next event from the handler thread.
    ///
    /// This function will always block the current thread if
    ///
    /// there is no data available and it's possible for more data to be sent.
    pub fn next(&self) -> Result<TerminalEvent> {
        Ok(self.receiver.recv()?)
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
