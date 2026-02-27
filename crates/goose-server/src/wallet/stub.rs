use super::{PaymentReceivedEvent, WalletBalance, WalletState};
use tokio::sync::broadcast;

/// No-op wallet manager used when the `lightning` feature is not enabled.
#[derive(Clone)]
pub struct WalletManager {
    _tx: broadcast::Sender<PaymentReceivedEvent>,
}

impl Default for WalletManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self { _tx: tx }
    }

    pub fn auto_start_if_configured(self: &std::sync::Arc<Self>) {}

    pub async fn get_state(&self) -> WalletState {
        WalletState::Disabled
    }

    pub async fn get_balance(&self) -> anyhow::Result<WalletBalance> {
        anyhow::bail!("Lightning wallet is not enabled — rebuild with --features lightning")
    }

    pub async fn create_invoice(
        &self,
        _amount_sats: Option<u64>,
    ) -> anyhow::Result<super::Invoice> {
        anyhow::bail!("Lightning wallet is not enabled — rebuild with --features lightning")
    }

    pub async fn parse_invoice(&self, _bolt11: &str) -> anyhow::Result<super::ParsedInvoice> {
        anyhow::bail!("Lightning wallet is not enabled — rebuild with --features lightning")
    }

    pub async fn pay_invoice(&self, _bolt11: &str) -> anyhow::Result<(u64, String)> {
        anyhow::bail!("Lightning wallet is not enabled — rebuild with --features lightning")
    }

    pub fn subscribe_payments(&self) -> broadcast::Receiver<PaymentReceivedEvent> {
        self._tx.subscribe()
    }

    #[allow(dead_code)]
    pub async fn stop(&self) {}
}
