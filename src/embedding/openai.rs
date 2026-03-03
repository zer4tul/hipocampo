//! OpenAI embedding provider

use crate::embedding::EmbeddingProvider;
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// OpenAI embedding models
#[derive(Debug, Clone)]
pub enum OpenAIModel {
    TextEmbedding3Small,
    TextEmbedding3Large,
    TextEmbeddingAda002,
}

impl OpenAIModel {
    pub fn dimensions(&self) -> usize {
        match self {
            Self::TextEmbedding3Small => 1536,
            Self::TextEmbedding3Large => 3072,
            Self::TextEmbeddingAda002 => 1536,
        }
    }

    pub fn model_name(&self) -> &str {
        match self {
            Self::TextEmbedding3Small => "text-embedding-3-small",
            Self::TextEmbedding3Large => "text-embedding-3-large",
            Self::TextEmbeddingAda002 => "text-embedding-ada-002",
        }
    }
}

/// OpenAI embedding provider
pub struct OpenAIEmbedding {
    client: Client,
    api_key: String,
    model: OpenAIModel,
}

impl OpenAIEmbedding {
    pub fn new(api_key: String, model: OpenAIModel) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub fn from_env(model: OpenAIModel) -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        Ok(Self::new(api_key, model))
    }
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbedding {
    fn name(&self) -> &str {
        "openai"
    }

    fn dimensions(&self) -> usize {
        self.model.dimensions()
    }

    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let request = EmbeddingRequest {
            model: self.model.model_name().to_string(),
            input: texts.iter().map(|s| s.to_string()).collect(),
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?
            .json::<EmbeddingResponse>()
            .await?;

        let mut embeddings = response.data;
        embeddings.sort_by(|a, b| a.embedding.len().cmp(&b.embedding.len()));

        Ok(embeddings.into_iter().map(|d| d.embedding).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_dimensions() {
        assert_eq!(OpenAIModel::TextEmbedding3Small.dimensions(), 1536);
        assert_eq!(OpenAIModel::TextEmbedding3Large.dimensions(), 3072);
        assert_eq!(OpenAIModel::TextEmbeddingAda002.dimensions(), 1536);
    }
}
