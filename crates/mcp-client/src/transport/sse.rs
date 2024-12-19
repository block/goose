use super::{Error, Transport};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::{Client, Url};
use reqwest_eventsource::{Event, EventSource};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info};

pub struct SseTransport {
    connection_url: Url,
    endpoint: Arc<Mutex<Option<Url>>>,
    http_client: Client,
    event_source: Arc<Mutex<Option<EventSource>>>,
    message_rx: Arc<Mutex<Option<mpsc::Receiver<String>>>>,
    message_tx: mpsc::Sender<String>,
}

impl SseTransport {
    pub fn new(url: &str) -> Result<Self, Error> {
        let (message_tx, message_rx) = mpsc::channel(100);

        Ok(Self {
            connection_url: Url::parse(url).map_err(|_| Error::InvalidUrl)?,
            endpoint: Arc::new(Mutex::new(None)),
            http_client: Client::new(),
            event_source: Arc::new(Mutex::new(None)),
            message_rx: Arc::new(Mutex::new(Some(message_rx))),
            message_tx,
        })
    }
}

/// Constructs the endpoint URL by removing "/sse" from the connection URL
/// and appending the given suffix.
fn construct_endpoint_url(base_url: &Url, url_suffix: &str) -> Result<Url, url::ParseError> {
    let trimmed_base = base_url.as_str().trim_end_matches("/sse");
    let trimmed_base = trimmed_base.trim_end_matches('/');
    let trimmed_suffix = url_suffix.trim_start_matches('/');
    let full_url = format!("{}/{}", trimmed_base, trimmed_suffix);
    Url::parse(&full_url)
}

#[async_trait]
impl Transport for SseTransport {
    async fn start(&self) -> Result<(), Error> {
        if self.event_source.lock().await.is_some() {
            return Ok(());
        }

        let event_source = EventSource::get(self.connection_url.as_str());
        let message_tx = self.message_tx.clone();
        let endpoint = self.endpoint.clone();

        // Store event source
        *self.event_source.lock().await = Some(event_source);

        // Create a new event source for the task
        let mut stream = EventSource::get(self.connection_url.as_str());

        let connection_url = self.connection_url.clone();
        let cloned_connection_url = connection_url.clone();

        // Spawn a task to handle incoming events
        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                match event {
                    Ok(Event::Open) => {
                        // Connection established
                        info!("\nSSE connection opened");
                    }
                    Ok(Event::Message(message)) => {
                        debug!("Received SSE event: {} - {}", message.event, message.data);
                        // Check if this is an endpoint event
                        if message.event == "endpoint" {
                            let url_suffix = &message.data;
                            debug!("Received endpoint URL suffix: {}", url_suffix);
                            match construct_endpoint_url(&cloned_connection_url, url_suffix) {
                                Ok(url) => {
                                    info!("Endpoint URL: {}", url);
                                    let mut endpoint_guard = endpoint.lock().await;
                                    *endpoint_guard = Some(url);
                                }
                                Err(e) => {
                                    error!("Failed to construct endpoint URL: {}", e);
                                    // Optionally, handle the error (e.g., retry, notify, etc.)
                                }
                            }
                        } else {
                            // Regular message
                            // Assuming message.data is the message payload
                            if let Err(e) = message_tx.send(message.data).await {
                                error!("Failed to send message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("EventSource error: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for endpoint URL: every 100ms, check if the endpoint is set upto 30s timeout
        let timeout = tokio::time::sleep(std::time::Duration::from_secs(30));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    return Err(Error::Timeout);
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    let endpoint_guard = self.endpoint.lock().await;
                    if endpoint_guard.is_some() {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn send(&self, msg: String) -> Result<(), Error> {
        let endpoint = {
            let endpoint_guard = self.endpoint.lock().await;
            endpoint_guard.as_ref().ok_or(Error::NotConnected)?.clone()
        };

        self.http_client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .body(msg)
            .send()
            .await
            .map_err(|_| Error::SendFailed)?;

        Ok(())
    }

    async fn receive(&self) -> Result<String, Error> {
        let mut rx_guard = self.message_rx.lock().await;
        let rx = rx_guard.as_mut().ok_or(Error::NotConnected)?;

        rx.recv().await.ok_or(Error::NotConnected)
    }

    async fn close(&self) -> Result<(), Error> {
        *self.event_source.lock().await = None;
        *self.endpoint.lock().await = None;
        Ok(())
    }
}
