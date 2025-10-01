use anyhow::Result;
use serde::Deserialize;
use std::path::Path;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub document: String,
    pub uuid: String,
    pub text: String,
    pub chunk_index: u32,
}

#[derive(Debug)]
pub struct Embed {
    pub embedding: Vec<f32>,
    pub chunk: Chunk,
    pub created_at: i64,
}

impl Embed {
    pub fn new(embedding: Vec<f32>, chunk: Chunk) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self {
            embedding,
            chunk,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    // pub fn mock(n: usize) -> Self {
    //     let chunk = Chunk::new("hi.pdf".to_owned(), "A lot of text".to_owned(), 30);
    //     let embedding = vec![0.0_f32; n];
    //     Embed::new(embedding, chunk)
    // }
}

impl Chunk {
    pub fn new(document: String, text: String, chunk_index: u32) -> Self {
        Self {
            document,
            uuid: uuid::Uuid::now_v7().to_string(),
            text,
            chunk_index,
        }
    }

    pub async fn embed(&self, endpoint: &AI) -> Result<Embed> {
        let response = endpoint.post(&self.text).await.unwrap();
        let embed = Embed::new(response.embeddings[0].clone(), self.clone());
        Ok(embed)
    }
    
    /// Given a PDF path, we extract the chunks
    pub fn from_pdf<P: AsRef<Path>>(path: P) -> Result<Vec<Self>> {
        let pdf_text = pdf_extract::extract_text(path.as_ref())?;
        let document_name = path.as_ref()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown_document")
            .to_string();
        
        let chunks = chunk_text(&pdf_text, &document_name, 1024, 128);
        Ok(chunks)
    }
}

/// Split text into overlapping chunks (for embeddings / RAG).
fn chunk_text(text: &str, document: &str, chunk_size: usize, overlap: usize) -> Vec<Chunk> {
    let words: Vec<&str> = text.unicode_words().collect();
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut chunk_index = 0;

    while start < words.len() {
        let end = (start + chunk_size).min(words.len());
        let chunk_text = words[start..end].join(" ");
        
        let chunk = Chunk::new(            
            document.to_string(),
            chunk_text,
            chunk_index,
        );
        
        chunks.push(chunk);
        
        if end == words.len() {
            break;
        }
        
        start += chunk_size - overlap;
        chunk_index += 1;
    }

    chunks
}

#[derive(Debug, Clone)]
pub enum Provider {
    Ollama,
}

pub const EMBEDDING_MODELS: [&'static str; 11] = [
    "embeddinggemma",
    "qwen3-embedding",
    "granite-embedding",
    "snowflake-arctic-embed2",
    "bge-large",
    "paraphrase-multilingual",
    "bge-m3",
    "snowflake-arctic-embed",
    "mxbai-embed-large",
    "all-minilm",
    "nomic-embed-text",
];

#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    pub model: String,
    pub embeddings: Vec<Vec<f32>>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u32>,
}

#[derive(Clone)]
pub struct AI {
    pub url: String,
    pub model: String,
    pub provider: Provider,
    pub dims: usize,
}

impl AI {
    pub fn new(url: &str, model: &str, dims: usize) -> Self {
        return Self {
            url: url.to_owned(),
            model: model.to_owned(),
            provider: Provider::Ollama,
            dims
        };
    }

    // Here we should do different implementations based on provider
    pub async fn post(&self, input: &str) -> Result<EmbedResponse> {
        let response = reqwest::Client::new()
            .post(&self.url)
            .json(&serde_json::json!({
                "model": self.model,
                "input": input,
            }))
            .send()
            .await?
            .json::<EmbedResponse>()
            .await?;

        Ok(response)
    }

    pub async fn get_dim(&self) -> Result<usize> {
        let response = Self::post(
            self,
            "Just anything to get the dimensions of the embedding model ",
        )
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?; // force conversion
        
        Ok(response.embeddings[0].len())
    }
}
