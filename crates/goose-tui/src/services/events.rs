use crate::services::client::ProviderDetails;
use crate::state::ToolInfo;
use crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};
use futures::{FutureExt, StreamExt};
use goose::config::ExtensionEntry;
use goose::session::Session;
use goose_server::routes::reply::MessageEvent;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Event {
    Input(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    Tick,
    Resize,
    Server(Arc<MessageEvent>),
    SessionsList(Vec<Session>),
    SessionResumed(Box<Session>),
    ToolsLoaded(Vec<ToolInfo>),
    ProvidersLoaded(Vec<ProviderDetails>),
    ExtensionsLoaded(Vec<ExtensionEntry>),
    ModelsLoaded {
        provider: String,
        models: Vec<String>,
    },
    ConfigLoaded(serde_json::Value),
    Error(String),
}
#[allow(dead_code)]
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    tx: mpsc::UnboundedSender<Event>,
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();

        tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick = tokio::time::interval(Duration::from_millis(30));

            loop {
                let tick_delay = tick.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = tick_delay => {
                        let _ = _tx.send(Event::Tick);
                    }
                    Some(Ok(evt)) = crossterm_event => {
                        match evt {
                            CrosstermEvent::Key(key) => {
                                if key.kind == KeyEventKind::Press {
                                    let _ = _tx.send(Event::Input(key));
                                }
                            }
                            CrosstermEvent::Mouse(mouse) => {
                                let _ = _tx.send(Event::Mouse(mouse));
                            }
                            CrosstermEvent::Resize(_w, _h) => {
                                let _ = _tx.send(Event::Resize);
                            }
                            CrosstermEvent::Paste(s) => {
                                let _ = _tx.send(Event::Paste(s));
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Self { rx, tx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub fn try_next(&mut self) -> Option<Event> {
        self.rx.try_recv().ok()
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }
}
