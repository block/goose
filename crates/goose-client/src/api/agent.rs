use crate::error::Result;
use crate::types::requests::{
    AddExtensionRequest, ImportAppRequest, ReadResourceRequest, RemoveExtensionRequest,
    RestartAgentRequest, ResumeAgentRequest, SetContainerRequest, StartAgentRequest,
    StopAgentRequest, UpdateFromSessionRequest, UpdateProviderRequest, UpdateWorkingDirRequest,
};
use crate::types::responses::{
    ImportAppResponse, ListAppsResponse, ReadResourceResponse, RestartAgentResponse,
    ResumeAgentResponse,
};
use crate::GooseClient;
use goose::agents::ExtensionConfig;
use goose::session::Session;

impl GooseClient {
    pub async fn start_agent(&self, request: StartAgentRequest) -> Result<Session> {
        self.http.post("/agent/start", &request).await
    }

    pub async fn resume_agent(
        &self,
        session_id: impl Into<String>,
        load_model_and_extensions: bool,
    ) -> Result<ResumeAgentResponse> {
        self.http
            .post(
                "/agent/resume",
                &ResumeAgentRequest {
                    session_id: session_id.into(),
                    load_model_and_extensions,
                },
            )
            .await
    }

    pub async fn stop_agent(&self, session_id: impl Into<String>) -> Result<()> {
        self.http
            .post_empty(
                "/agent/stop",
                &StopAgentRequest {
                    session_id: session_id.into(),
                },
            )
            .await
    }

    pub async fn restart_agent(
        &self,
        session_id: impl Into<String>,
    ) -> Result<RestartAgentResponse> {
        self.http
            .post(
                "/agent/restart",
                &RestartAgentRequest {
                    session_id: session_id.into(),
                },
            )
            .await
    }

    pub async fn update_working_dir(
        &self,
        session_id: impl Into<String>,
        working_dir: impl Into<String>,
    ) -> Result<()> {
        self.http
            .post_empty(
                "/agent/update_working_dir",
                &UpdateWorkingDirRequest {
                    session_id: session_id.into(),
                    working_dir: working_dir.into(),
                },
            )
            .await
    }

    pub async fn update_from_session(&self, session_id: impl Into<String>) -> Result<()> {
        self.http
            .post_empty(
                "/agent/update_from_session",
                &UpdateFromSessionRequest {
                    session_id: session_id.into(),
                },
            )
            .await
    }

    pub async fn update_provider(&self, request: UpdateProviderRequest) -> Result<()> {
        self.http
            .post_empty("/agent/update_provider", &request)
            .await
    }

    pub async fn add_extension(
        &self,
        session_id: impl Into<String>,
        config: ExtensionConfig,
    ) -> Result<()> {
        self.http
            .post_empty(
                "/agent/add_extension",
                &AddExtensionRequest {
                    session_id: session_id.into(),
                    config,
                },
            )
            .await
    }

    pub async fn remove_extension(
        &self,
        session_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<()> {
        self.http
            .post_empty(
                "/agent/remove_extension",
                &RemoveExtensionRequest {
                    session_id: session_id.into(),
                    name: name.into(),
                },
            )
            .await
    }

    pub async fn read_resource(
        &self,
        request: ReadResourceRequest,
    ) -> Result<ReadResourceResponse> {
        self.http.post("/agent/read_resource", &request).await
    }

    pub async fn list_apps(&self, session_id: Option<&str>) -> Result<ListAppsResponse> {
        match session_id {
            Some(id) => {
                self.http
                    .get_with_query("/agent/list_apps", &[("session_id", id)])
                    .await
            }
            None => self.http.get("/agent/list_apps").await,
        }
    }

    pub async fn export_app(&self, name: impl AsRef<str>) -> Result<String> {
        self.http
            .get_text(&format!("/agent/export_app/{}", name.as_ref()))
            .await
    }

    pub async fn import_app(&self, html: impl Into<String>) -> Result<ImportAppResponse> {
        self.http
            .post("/agent/import_app", &ImportAppRequest { html: html.into() })
            .await
    }

    pub async fn set_container(
        &self,
        session_id: impl Into<String>,
        container_id: Option<String>,
    ) -> Result<()> {
        self.http
            .post_empty(
                "/agent/set_container",
                &SetContainerRequest {
                    session_id: session_id.into(),
                    container_id,
                },
            )
            .await
    }
}
