use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use futures::{FutureExt, StreamExt};
use goose_server::routes::reply::MessageEvent;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    Input(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    Resize(u16, u16),
    Server(MessageEvent),
    Error(String),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    tx: mpsc::UnboundedSender<Event>,
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_for_task = tx.clone();

        let cancellation_token = CancellationToken::new();
        let _cancel_token = cancellation_token.clone();

        let task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                let tick_delay = tick_interval.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = _cancel_token.cancelled() => {
                        break;
                    }
                    _ = tick_delay => {
                        if tx_for_task.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    Some(Ok(evt)) = crossterm_event => {
                        match evt {
                            CrosstermEvent::Key(key) => {
                                if key.kind == event::KeyEventKind::Press {
                                    if tx_for_task.send(Event::Input(key)).is_err() {
                                        break;
                                    }
                                }
                            }
                            CrosstermEvent::Mouse(mouse) => {
                                if tx_for_task.send(Event::Mouse(mouse)).is_err() {
                                    break;
                                }
                            }
                            CrosstermEvent::Resize(x, y) => {
                                if tx_for_task.send(Event::Resize(x, y)).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Self {
            rx,
            tx,
            _task: task,
        }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }
}
