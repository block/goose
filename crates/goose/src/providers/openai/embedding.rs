use super::*;

#[async_trait]
impl EmbeddingCapable for OpenAiProvider {
    async fn create_embeddings(
        &self,
        session_id: &str,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let embedding_model = std::env::var("GOOSE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        let request = EmbeddingRequest {
            input: texts,
            model: embedding_model,
        };

        let response = self
            .with_retry(|| async {
                let request_clone = EmbeddingRequest {
                    input: request.input.clone(),
                    model: request.model.clone(),
                };
                let request_value = serde_json::to_value(request_clone)
                    .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
                let embeddings_path = OpenAiProvider::map_base_path(
                    &self.base_path,
                    "embeddings",
                    OPEN_AI_DEFAULT_EMBEDDINGS_PATH,
                );
                self.api_client
                    .api_post(Some(session_id), &embeddings_path, &request_value)
                    .await
                    .map_err(|e| ProviderError::ExecutionError(e.to_string()))
            })
            .await?;

        if response.status != StatusCode::OK {
            let error_text = response
                .payload
                .as_ref()
                .and_then(|p| p.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow::anyhow!("Embedding API error: {}", error_text));
        }

        let embedding_response: EmbeddingResponse = serde_json::from_value(
            response
                .payload
                .ok_or_else(|| anyhow::anyhow!("Empty response body"))?,
        )?;

        Ok(embedding_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }
}
