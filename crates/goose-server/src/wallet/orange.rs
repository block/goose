use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, Mutex, RwLock};

use goose::config::paths::Paths;
use goose::config::Config;
use orange_sdk::bitcoin::Network;
use orange_sdk::bitcoin_payment_instructions::amount::Amount;
use orange_sdk::{
    ChainSource, Event, ExtraConfig, LoggerType, Mnemonic, Seed, SparkWalletConfig, StorageConfig,
    Tunables, Wallet, WalletConfig,
};

use super::{Invoice, PaymentDirection, PaymentReceivedEvent, PaymentRecord, PaymentStatus, WalletBalance, WalletState};
use orange_sdk::TxStatus;

/// Map of in-flight outgoing payments awaiting their preimage.
type PendingPayments = Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>;

/// Manages the Orange SDK wallet lifecycle.
///
/// The wallet is lazily initialized on first use. Configuration (seed, network,
/// electrum URL) is read from Goose's `Config` system.
#[derive(Clone)]
pub struct WalletManager {
    wallet: Arc<RwLock<Option<Wallet>>>,
    state: Arc<RwLock<WalletState>>,
    tx: broadcast::Sender<PaymentReceivedEvent>,
    /// Pending outgoing payments keyed by PaymentId string, awaiting preimage.
    pending_payments: PendingPayments,
}

impl Default for WalletManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletManager {
    /// Create a new, uninitialized wallet manager.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            wallet: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(WalletState::Uninitialized)),
            tx,
            pending_payments: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// If a seed already exists in the keyring, start the wallet in the background.
    pub fn auto_start_if_configured(self: &Arc<Self>) {
        let config = Config::global();
        if config.get_secret::<String>("ORANGE_WALLET_SEED").is_ok() {
            tracing::info!("Existing wallet seed found — starting wallet");
            let mgr = Arc::clone(self);
            tokio::spawn(async move {
                if let Err(e) = mgr.ensure_initialized().await {
                    tracing::warn!("Wallet auto-start failed: {e:#}");
                }
            });
        }
    }

    /// Return the current wallet state.
    pub async fn get_state(&self) -> WalletState {
        self.state.read().await.clone()
    }

    /// Lazily initialize the wallet if not already running.
    ///
    /// Reads seed from the keyring (`ORANGE_WALLET_SEED`), network and electrum
    /// config from `Config::get_param`, then starts the Orange SDK wallet and
    /// spawns the event loop.
    pub async fn ensure_initialized(&self) -> anyhow::Result<()> {
        // Fast path: already initialized.
        {
            let state = self.state.read().await;
            match &*state {
                WalletState::Ready => return Ok(()),
                WalletState::Initializing => {
                    anyhow::bail!("Wallet is currently initializing, please wait")
                }
                WalletState::Disabled => {
                    anyhow::bail!("Lightning wallet is disabled")
                }
                _ => {}
            }
        }

        // Mark as initializing.
        {
            let mut state = self.state.write().await;
            *state = WalletState::Initializing;
        }

        match self.do_init().await {
            Ok(()) => {
                let mut state = self.state.write().await;
                *state = WalletState::Ready;
                Ok(())
            }
            Err(e) => {
                let msg = format!("{e:#}");
                let mut state = self.state.write().await;
                *state = WalletState::Error {
                    message: msg.clone(),
                };
                anyhow::bail!("Wallet initialization failed: {msg}")
            }
        }
    }

    async fn do_init(&self) -> anyhow::Result<()> {
        // Ensure rustls crypto provider is installed (needed by LDK node internals).
        let _ = rustls::crypto::ring::default_provider().install_default();

        let config = Config::global();

        // Read mnemonic from keyring/env, or generate a new one on first use.
        let mnemonic = match config.get_secret::<String>("ORANGE_WALLET_SEED") {
            Ok(mnemonic_str) => Mnemonic::parse(&mnemonic_str)
                .map_err(|e| anyhow::anyhow!("Invalid mnemonic in keyring: {e}"))?,
            Err(_) => {
                tracing::info!("No wallet seed found — generating a new one");
                let mnemonic = generate_mnemonic()?;
                config
                    .set_secret("ORANGE_WALLET_SEED", &mnemonic.to_string())
                    .map_err(|e| anyhow::anyhow!("Failed to store wallet seed: {e}"))?;
                tracing::info!("New wallet seed stored in keyring");
                mnemonic
            }
        };

        // Network (default: signet).
        let network_str: String = config
            .get_param("orange_network")
            .unwrap_or_else(|_| "bitcoin".to_string());
        let network = match network_str.as_str() {
            "bitcoin" | "mainnet" => Network::Bitcoin,
            "signet" => Network::Signet,
            "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            other => anyhow::bail!("Unknown network: {other}"),
        };

        // Chain source.
        let chain_source = ChainSource::Esplora {
            url: config
                .get_param("orange_esplora_url")
                .unwrap_or_else(|_| "https://blockstream.info/api".to_string()),
            username: None,
            password: None,
        };

        // LSP configuration.
        let lsp_address_str: String = config
            .get_param("orange_lsp_address")
            .unwrap_or_else(|_| "69.59.18.144:9735".to_string());
        let lsp_pubkey_str: String = config.get_param("orange_lsp_pubkey").unwrap_or_else(|_| {
            "021deaa26ce6bb7cc63bd30e83a2bba1c0368269fa3bb9b616a24f40d941ac7d32".to_string()
        });
        let lsp_token: Option<String> = config
            .get_param("orange_lsp_token")
            .ok()
            .or_else(|| Some("DeveloperTestingOnly".to_string()));

        // Parse LSP address and pubkey — use .parse() to let Rust infer
        // the target types from the WalletConfig struct definition.
        let lsp_address = lsp_address_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid LSP address: {lsp_address_str}"))?;
        let lsp_pubkey = lsp_pubkey_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid LSP pubkey: {e}"))?;

        // Storage path.
        let data_dir = Paths::data_dir();
        let db_path = data_dir.join("orange_wallet.db");
        std::fs::create_dir_all(&data_dir)?;

        let log_path = data_dir.join("orange_wallet.log");

        let wallet_config = WalletConfig {
            storage_config: StorageConfig::LocalSQLite(db_path.to_string_lossy().to_string()),
            logger_type: LoggerType::File { path: log_path },
            chain_source,
            lsp: (lsp_address, lsp_pubkey, lsp_token),
            scorer_url: None,
            rgs_url: None,
            network,
            seed: Seed::Mnemonic {
                mnemonic,
                passphrase: None,
            },
            tunables: Tunables::default(),
            extra_config: ExtraConfig::Spark(SparkWalletConfig::default()),
        };

        let wallet = Wallet::new(wallet_config)
            .await
            .map_err(|e| anyhow::anyhow!("Orange SDK init failed: {e:?}"))?;

        // Spawn event loop before storing the wallet.
        self.spawn_event_loop(wallet.clone());

        {
            let mut w = self.wallet.write().await;
            *w = Some(wallet);
        }

        tracing::info!("Orange wallet initialized successfully");
        Ok(())
    }

    /// Spawn a background task that polls `wallet.next_event_async()` and
    /// broadcasts payment-received events over the channel.
    fn spawn_event_loop(&self, wallet: Wallet) {
        let tx = self.tx.clone();
        let pending = self.pending_payments.clone();

        tokio::spawn(async move {
            loop {
                let event = wallet.next_event_async().await;
                match &event {
                    Event::PaymentReceived {
                        payment_hash,
                        amount_msat,
                        ..
                    } => {
                        let hash_hex = hex::encode(payment_hash.0);
                        let sats = amount_msat / 1000;
                        let evt = PaymentReceivedEvent {
                            amount_msats: *amount_msat,
                            amount_sats: sats,
                            payment_hash: hash_hex,
                        };
                        tracing::info!(amount_sats = sats, "Lightning payment received");
                        let _ = tx.send(evt);
                    }
                    Event::OnchainPaymentReceived {
                        amount_sat, txid, ..
                    } => {
                        let evt = PaymentReceivedEvent {
                            amount_msats: amount_sat * 1000,
                            amount_sats: *amount_sat,
                            payment_hash: txid.to_string(),
                        };
                        tracing::info!(amount_sats = evt.amount_sats, "On-chain payment received");
                        let _ = tx.send(evt);
                    }
                    Event::PaymentSuccessful {
                        payment_id,
                        payment_preimage,
                        ..
                    } => {
                        let key = payment_id.to_string();
                        let preimage_hex = hex::encode(payment_preimage.0);
                        tracing::info!(%key, "Outgoing payment successful");
                        let mut map = pending.lock().await;
                        if let Some(sender) = map.remove(&key) {
                            let _ = sender.send(preimage_hex);
                        }
                    }
                    Event::PaymentFailed { payment_id, .. } => {
                        let key = payment_id.to_string();
                        tracing::warn!(%key, "Outgoing payment failed");
                        // Dropping the sender causes the receiver to get an error.
                        let mut map = pending.lock().await;
                        map.remove(&key);
                    }
                    other => {
                        tracing::debug!(?other, "Orange wallet event");
                    }
                }
                // Acknowledge the event so the SDK can proceed.
                let _ = wallet.event_handled();
            }
        });
    }

    /// Get the current wallet balance.
    pub async fn get_balance(&self) -> anyhow::Result<WalletBalance> {
        self.ensure_initialized().await?;
        let wallet = self.wallet.read().await;
        let wallet = wallet
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Wallet not initialized"))?;

        let balances = wallet
            .get_balance()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get balance: {e:?}"))?;

        let trusted = balances.trusted.sats_rounding_up();
        let lightning = balances.lightning.sats_rounding_up();
        let pending = balances.pending_balance.sats_rounding_up();

        Ok(WalletBalance {
            trusted_sats: trusted,
            lightning_sats: lightning,
            pending_sats: pending,
            total_sats: trusted + lightning,
        })
    }

    /// Create a Lightning invoice for the given amount.
    pub async fn create_invoice(&self, amount_sats: Option<u64>) -> anyhow::Result<Invoice> {
        self.ensure_initialized().await?;
        let wallet = self.wallet.read().await;
        let wallet = wallet
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Wallet not initialized"))?;

        let amount = match amount_sats {
            Some(sats) => Some(
                Amount::from_sats(sats)
                    .map_err(|_| anyhow::anyhow!("Invalid amount: {sats} sats"))?,
            ),
            None => None,
        };

        let uri = wallet
            .get_single_use_receive_uri(amount)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create invoice: {e:?}"))?;

        let bolt11 = uri.invoice.to_string();

        // Generate QR code SVG.
        let qr_svg = generate_qr_svg(&bolt11)?;

        Ok(Invoice {
            bolt11,
            qr_svg,
            amount_sats,
        })
    }

    /// Parse a BOLT11 invoice and return its details without paying.
    pub async fn parse_invoice(&self, bolt11: &str) -> anyhow::Result<super::ParsedInvoice> {
        self.ensure_initialized().await?;
        let wallet = self.wallet.read().await;
        let wallet = wallet
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Wallet not initialized"))?;

        let instructions = wallet
            .parse_payment_instructions(bolt11)
            .await
            .map_err(|e| anyhow::anyhow!("Invalid invoice: {e:?}"))?;

        let amount_sats = orange_sdk::PaymentInfo::build(instructions, None)
            .ok()
            .map(|pi| pi.amount().sats_rounding_up());

        // Try to extract description from the BOLT11 string directly.
        let description = None; // BOLT11 description parsing would require additional deps

        Ok(super::ParsedInvoice {
            amount_sats,
            description,
        })
    }

    /// Pay a BOLT11 invoice. If `user_amount_sats` is provided it is used for
    /// amountless invoices, Lightning addresses, etc.  Returns `(amount_sats, preimage_hex)`.
    pub async fn pay_invoice(
        &self,
        bolt11: &str,
        user_amount_sats: Option<u64>,
    ) -> anyhow::Result<(u64, String)> {
        self.ensure_initialized().await?;

        // Create a oneshot channel to receive the preimage from the event loop.
        let (preimage_tx, preimage_rx) = oneshot::channel::<String>();

        let amount_sats;
        {
            let wallet = self.wallet.read().await;
            let wallet = wallet
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Wallet not initialized"))?;

            let instructions = wallet
                .parse_payment_instructions(bolt11)
                .await
                .map_err(|e| anyhow::anyhow!("Invalid payment instructions: {e:?}"))?;

            let sdk_amount = match user_amount_sats {
                Some(sats) => Some(
                    Amount::from_sats(sats)
                        .map_err(|_| anyhow::anyhow!("Invalid amount: {sats} sats"))?,
                ),
                None => None,
            };

            let payment_info = orange_sdk::PaymentInfo::build(instructions, sdk_amount)
                .map_err(|e| anyhow::anyhow!("Failed to build payment: {e:?}"))?;

            amount_sats = payment_info.amount().sats_rounding_up();

            let payment_id = wallet
                .pay(&payment_info)
                .await
                .map_err(|e| anyhow::anyhow!("Payment failed: {e:?}"))?;

            // Register the oneshot so the event loop can resolve it.
            let key = payment_id.to_string();
            tracing::info!(%key, amount_sats, "Lightning payment initiated, waiting for confirmation");
            self.pending_payments.lock().await.insert(key, preimage_tx);
        }

        // Wait for the event loop to deliver the preimage.
        let preimage_hex = preimage_rx
            .await
            .map_err(|_| anyhow::anyhow!("Payment failed — no preimage received"))?;

        tracing::info!(amount_sats, "Lightning payment confirmed");
        Ok((amount_sats, preimage_hex))
    }

    /// Return the payment history from the wallet's persisted store (newest first).
    pub async fn get_history(&self) -> anyhow::Result<Vec<PaymentRecord>> {
        self.ensure_initialized().await?;
        let wallet = self.wallet.read().await;
        let wallet = wallet
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Wallet not initialized"))?;

        let txs = match wallet.list_transactions().await {
            Ok(txs) => txs,
            Err(e) => {
                tracing::warn!("list_transactions failed, returning empty history: {e:?}");
                return Ok(Vec::new());
            }
        };

        tracing::debug!(
            total = txs.len(),
            completed = txs.iter().filter(|tx| tx.status == TxStatus::Completed).count(),
            pending = txs.iter().filter(|tx| tx.status == TxStatus::Pending).count(),
            failed = txs.iter().filter(|tx| tx.status == TxStatus::Failed).count(),
            "list_transactions breakdown"
        );

        let mut records: Vec<PaymentRecord> = txs
            .into_iter()
            .filter(|tx| tx.status != TxStatus::Failed)
            .map(|tx| {
                let direction = if tx.outbound {
                    PaymentDirection::Outgoing
                } else {
                    PaymentDirection::Incoming
                };
                let status = if tx.status == TxStatus::Completed {
                    PaymentStatus::Completed
                } else {
                    PaymentStatus::Pending
                };
                let amount_sats = tx.amount.map(|a| a.sats_rounding_up()).unwrap_or(0);
                let payment_hash = tx.id.to_string();
                let timestamp = tx.time_since_epoch.as_secs();

                PaymentRecord {
                    direction,
                    status,
                    amount_sats,
                    payment_hash,
                    timestamp,
                    description: None,
                }
            })
            .collect();

        // Sort newest first.
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(records)
    }

    /// Subscribe to payment received events for SSE streaming.
    pub fn subscribe_payments(&self) -> broadcast::Receiver<PaymentReceivedEvent> {
        self.tx.subscribe()
    }

    /// Gracefully shut down the wallet.
    #[allow(dead_code)]
    pub async fn stop(&self) {
        let wallet = self.wallet.read().await;
        if let Some(w) = wallet.as_ref() {
            w.stop().await;
        }
    }
}

/// Generate a new random 12-word BIP39 mnemonic.
fn generate_mnemonic() -> anyhow::Result<Mnemonic> {
    let mut entropy = [0u8; 16]; // 128 bits → 12 words
    rand::fill(&mut entropy);
    Mnemonic::from_entropy(&entropy)
        .map_err(|e| anyhow::anyhow!("Failed to create mnemonic from entropy: {e}"))
}

/// Generate a QR code as an SVG string from the given data.
fn generate_qr_svg(data: &str) -> anyhow::Result<String> {
    use qrcode::render::svg;
    use qrcode::QrCode;

    let code = QrCode::new(data.as_bytes())
        .map_err(|e| anyhow::anyhow!("QR code generation failed: {e}"))?;

    let svg_string = code
        .render::<svg::Color>()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();

    Ok(svg_string)
}
