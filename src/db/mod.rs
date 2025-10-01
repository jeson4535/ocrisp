pub mod data;
pub mod qdrant;
use std::path::PathBuf;
use anyhow::Result;
use qdrant_client::{
    Qdrant,
    qdrant::{CollectionOperationResponse, PointsOperationResponse, SearchResponse},
};

use crate::abstractions::*;
use crate::db::qdrant::Guard;

// The return values should be replaced by Enums, when a new database option is added
#[allow(async_fn_in_trait)]
pub trait DBTrait {
    fn init(url: Option<&str>) -> Result<Qdrant>;
    fn is_available(&self) -> bool;
    fn run_db(silent: bool) -> Result<Guard>;
    async fn create_table(
        &self,
        table_name: &str,
        vector_size: u64,
    ) -> Result<CollectionOperationResponse>;
    async fn post(&self, table_name: &str, embed: Embed) -> Result<PointsOperationResponse>;
    async fn search(
        &self,
        table_name: &str,
        vec: Vec<f32>,
        limit: Option<u64>,
    ) -> Result<SearchResponse>;
    fn search_to_str(search: SearchResponse) -> Vec<String>;

    /// we check if we embedded the pdf
    async fn has_pdf(&self, table_name: &str, pdf_name: &PathBuf, dims: usize) -> Result<bool>;
}

pub async fn embed_everything<DB: DBTrait>(db: &DB, endpoint: &AI) -> Result<()> {
    if !db.is_available() {
        return Err(anyhow::anyhow!("The database is not available"));
    }
    let vector_size = endpoint.get_dim().await?;
    let _ = db
        .create_table(&endpoint.model, vector_size as u64)
        .await
        .ok();

    let pdfs = crate::db::data::Data::list_pdfs();
    for pdf in pdfs {
        let chunks = Chunk::from_pdf(pdf)?;
        let mut i = 0;
        println!("{}", chunks.len());
        for chunk in chunks {
            println!("{}", i);
            i = i + 1;
            let embed = chunk.embed(endpoint).await?;
            let _ = db.post(&endpoint.model, embed).await?;
        }
    }

    println!("Everything has been embedded");
    Ok(())
}

/// We get an user's query and fetch the most relevant results from the database-
pub async fn search<DB: DBTrait>(
    db: &DB,
    endpoint: &AI,
    input: &str,
    limit: Option<u64>,
) -> Result<Vec<String>> {
    let response = endpoint.post(input).await?;
    let vec = response.embeddings[0].clone();
    let search = db.search(&endpoint.model, vec, limit).await?;
    return Ok(DB::search_to_str(search));
}

pub async fn simple_search<DB: DBTrait>(
    db: &DB,
    endpoint: &AI,
    input: &str,
    limit: Option<u64>,
) -> String {
    return nice_str(search(db, endpoint, input, limit).await.unwrap());    
}

pub fn nice_str(strs: Vec<String>) -> String{
    format!(
        "Search results (top {}):\n\n{}",
        strs.len(),
        strs.iter()
            .enumerate()
            .map(|(i, doc)| format!("{}. {}", i + 1, doc))
            .collect::<Vec<_>>()
            .join("\n\n")
    )
}