use crate::error::Result;
use crate::GooseClient;
use goose::session::SystemInfo;

impl GooseClient {
    pub async fn status(&self) -> Result<String> {
        self.http.get_text("/status").await
    }

    pub async fn system_info(&self) -> Result<SystemInfo> {
        self.http.get("/system_info").await
    }

    pub async fn diagnostics(&self, session_id: impl AsRef<str>) -> Result<Vec<u8>> {
        self.http
            .get_bytes(&format!("/diagnostics/{}", session_id.as_ref()))
            .await
    }
}
