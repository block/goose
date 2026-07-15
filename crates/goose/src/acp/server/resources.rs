use super::*;

fn resource_notification_session_id(notification: &ExtensionResourceNotification) -> &str {
    match notification {
        ExtensionResourceNotification::Updated { session_id, .. }
        | ExtensionResourceNotification::ListChanged { session_id, .. } => session_id,
    }
}

async fn should_forward_resource_notification(
    expected_session_id: &str,
    extension_manager: &crate::agents::ExtensionManager,
    notification: &ExtensionResourceNotification,
) -> bool {
    if resource_notification_session_id(notification) != expected_session_id {
        return false;
    }
    extension_manager
        .is_resource_notification_subscribed(notification)
        .await
}

impl GooseAcpAgent {
    async fn ensure_resource_notification_bridge(
        &self,
        session_id: &str,
        extension_manager: Arc<crate::agents::ExtensionManager>,
    ) -> Result<(), agent_client_protocol::Error> {
        let mut bridges = self.resource_notification_bridges.lock().await;
        if bridges.contains_key(session_id) {
            return Ok(());
        }
        let cx = self
            .client_cx
            .get()
            .ok_or_else(|| {
                agent_client_protocol::Error::internal_error().data("ACP client is not connected")
            })?
            .clone();
        let mut notifications = extension_manager.subscribe_resource_notifications();
        let bridge_manager = Arc::downgrade(&extension_manager);
        let expected_session_id = session_id.to_string();
        let bridge = tokio::spawn(async move {
            loop {
                let notification = match notifications.recv().await {
                    Ok(notification) => notification,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                let Some(bridge_manager) = bridge_manager.upgrade() else {
                    break;
                };
                if !should_forward_resource_notification(
                    &expected_session_id,
                    &bridge_manager,
                    &notification,
                )
                .await
                {
                    continue;
                }
                let result = match notification {
                    ExtensionResourceNotification::Updated {
                        session_id,
                        extension_name,
                        uri,
                    } => cx.send_notification(ResourceUpdatedNotification {
                        session_id,
                        extension_name,
                        uri,
                    }),
                    ExtensionResourceNotification::ListChanged {
                        session_id,
                        extension_name,
                    } => cx.send_notification(ResourceListChangedNotification {
                        session_id,
                        extension_name,
                    }),
                };
                if let Err(error) = result {
                    tracing::warn!(%error, "Failed to forward MCP resource notification over ACP");
                    break;
                }
            }
        });
        bridges.insert(session_id.to_string(), bridge);
        Ok(())
    }

    pub(super) async fn on_read_resource(
        &self,
        req: ReadResourceRequest,
    ) -> Result<ReadResourceResponse, agent_client_protocol::Error> {
        let session_id = &req.session_id;
        let agent = self.get_session_agent(&req.session_id).await?;
        self.ensure_resource_notification_bridge(
            &req.session_id,
            Arc::clone(&agent.extension_manager),
        )
        .await?;
        let cancel_token = CancellationToken::new();
        let result = agent
            .extension_manager
            .read_resource(session_id, &req.uri, &req.extension_name, cancel_token)
            .await
            .internal_err()?;
        let result_json = serde_json::to_value(&result).internal_err()?;
        Ok(ReadResourceResponse {
            result: result_json,
        })
    }

    pub(super) async fn on_subscribe_resource(
        &self,
        req: SubscribeResourceRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let agent = self.get_session_agent(&req.session_id).await?;
        self.ensure_resource_notification_bridge(
            &req.session_id,
            Arc::clone(&agent.extension_manager),
        )
        .await?;
        agent
            .extension_manager
            .subscribe_resource(
                &req.session_id,
                &req.extension_name,
                &req.uri,
                &req.subscriber_id,
            )
            .await
            .internal_err()?;
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_unsubscribe_resource(
        &self,
        req: UnsubscribeResourceRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let agent = self.get_session_agent(&req.session_id).await?;
        agent
            .extension_manager
            .unsubscribe_resource(
                &req.session_id,
                &req.extension_name,
                &req.uri,
                &req.subscriber_id,
            )
            .await
            .internal_err()?;
        Ok(EmptyResponse {})
    }
}
