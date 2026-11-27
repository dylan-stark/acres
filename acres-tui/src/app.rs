use color_eyre::Result;
use ratatui::prelude::Rect;
use tokio::sync::mpsc;

use crate::{
    action::Action,
    components::{Component, catalog::Fetcher, iiif::Iiif},
    config::{Config, get_data_dir},
    entry::{Entry, highlights},
    explorer::{Explorer, Source},
    failure::failed,
    saved::{self, SavedList},
    tui::{Event, Tui},
};

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl App {
    /// An app over the live collection, or over `from_file` when given.
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        from_file: Option<Vec<Entry>>,
        offline: bool,
    ) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        // One catalog for the whole app, so every request shares its rate limit and cache.
        let mut catalog = acres::Catalog::builder();
        if offline {
            catalog = catalog.offline(true);
        }
        let catalog = catalog.build()?;
        let config = Config::new()?;
        let saved = SavedList::open(get_data_dir().join(saved::FILE_NAME))?;
        let source = match from_file {
            Some(entries) => Source::File(entries),
            None => Source::Live {
                highlights: config.config.highlights.clone().unwrap_or_else(highlights),
            },
        };
        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![
                // The explorer owns the screen and the keys; the others only fetch.
                Box::new(
                    Explorer::new(source, &config)
                        .with_saved(saved)
                        .with_exports(get_data_dir().join("exports")),
                ),
                Box::new(Fetcher::new(action_tx.clone(), catalog.clone())),
                Box::new(Iiif::new(action_tx.clone(), catalog)),
            ],
            should_quit: false,
            should_suspend: false,
            config,
            action_tx,
            action_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            .mouse(self.config.config.mouse)
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }
        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            _ => {}
        }
        for component in self.components.iter_mut() {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::OpenUrl(ref url) => {
                    if let Err(error) = open_url(url) {
                        let error = error.wrap_err(format!("could not open {url}"));
                        self.action_tx
                            .send(Action::Failed(failed(error.as_ref())))?;
                    }
                }
                Action::CopyText(ref text) => {
                    if let Err(error) = copy_text(text) {
                        let error = error.wrap_err("could not copy the link");
                        self.action_tx
                            .send(Action::Failed(failed(error.as_ref())))?;
                    }
                }
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                _ => {}
            }
            for component in self.components.iter_mut() {
                if let Some(action) = component.update(action.clone())? {
                    self.action_tx.send(action)?
                };
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        let start = std::time::Instant::now();
        tui.draw(|frame| {
            for component in self.components.iter_mut() {
                if let Err(error) = component.draw(frame, frame.area()) {
                    let error = error.wrap_err("could not draw");
                    let _ = self.action_tx.send(Action::Failed(failed(error.as_ref())));
                }
            }
        })?;
        self.action_tx.send(Action::FrameTime(start.elapsed()))?;
        Ok(())
    }
}

/// Opens `url` with the platform's opener, in the background.
fn open_url(url: &str) -> Result<()> {
    let mut command = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    } else {
        std::process::Command::new("xdg-open")
    };
    command
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

/// Puts `text` on the clipboard with an OSC 52 escape, which most terminals support and which
/// works over SSH too, with no clipboard library.
fn copy_text(text: &str) -> Result<()> {
    use std::io::Write;
    let sequence = osc52(text);
    let mut stdout = std::io::stdout();
    stdout.write_all(sequence.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

/// The OSC 52 sequence that sets the clipboard to `text`.
fn osc52(text: &str) -> String {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(text);
    format!("\x1b]52;c;{encoded}\x07")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc52_sets_the_clipboard_to_base64_text() {
        assert_eq!(osc52("hi"), "\x1b]52;c;aGk=\x07");
    }
}
