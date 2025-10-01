use crate::abstractions::Embed;
use crate::db::DBTrait;
use anyhow::Result;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions;
use qdrant_client::qdrant::{Condition, Filter};
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointsOperationResponse, SearchResponse, VectorParamsBuilder,
};
use qdrant_client::qdrant::{PointStruct, UpsertPointsBuilder};
use qdrant_client::qdrant::{ScrollPoints, SearchPointsBuilder, WithPayloadSelector};
use std::path::PathBuf;
use std::process::Stdio;
use std::process::{Child, Command};

pub struct Guard {
    child: Child,
}

impl Guard {
    fn start(silent: bool) -> anyhow::Result<Self> {
        let mut command = Command::new("qdrant");
        
        if silent {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        
        let child = command.spawn()?;
        Ok(Self { child })
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        // Try to shut down Qdrant when the guard is dropped
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl DBTrait for Qdrant {
    fn init(url: Option<&str>) -> Result<Qdrant> {
        Ok(Qdrant::from_url(url.unwrap_or("http://localhost:6334")).build()?)
    }

    fn is_available(&self) -> bool {
        Command::new("qdrant").arg("--version").output().is_ok()
    }

    fn run_db(silent: bool) -> Result<Guard> {
        // let child = Command::new("qdrant").spawn()?; // start qdrant without waiting for it to finish
        // Ok(child)
        let guard = Guard::start(silent)?;
        Ok(guard)
    }

    async fn create_table(
        &self,
        table_name: &str,
        vector_size: u64,
    ) -> Result<qdrant_client::qdrant::CollectionOperationResponse> {
        let response = self
            .create_collection(
                CreateCollectionBuilder::new(table_name)
                    .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine)),
            )
            .await?;

        Ok(response)
    }

    async fn post(&self, table_name: &str, embed: Embed) -> Result<PointsOperationResponse> {
        let chunk = &embed.chunk;
        let point = PointStruct::new(
            chunk.uuid.clone(),
            embed.embedding.clone(),
            [
                ("chunk_index", ((chunk.chunk_index) as f32).into()),
                ("text", chunk.text.clone().into()),
                ("document", chunk.document.clone().into()),
                ("created_at", embed.created_at.into()),
            ],
        );

        let response = self
            .upsert_points(UpsertPointsBuilder::new(table_name, [point]))
            .await?;

        Ok(response)
    }

    async fn search(
        &self,
        table_name: &str,
        vector: Vec<f32>,
        limit: Option<u64>,
    ) -> Result<SearchResponse> {
        let limit = limit.unwrap_or(5);
        let search_request: SearchPointsBuilder =
            SearchPointsBuilder::new(table_name, vector, limit).with_payload(true);

        let response: SearchResponse = self.search_points(search_request).await?;

        Ok(response)
    }

    async fn has_pdf(&self, table_name: &str, pdf_name: &PathBuf, dims: usize) -> Result<bool> {
        let name = pdf_name.file_name().unwrap().to_str().unwrap().to_owned();
        let filter = Filter::must(vec![Condition::matches("document", name)]);

        // This is bad because we're computing similarity
        // todo: fix
        let search_request: SearchPointsBuilder =
            SearchPointsBuilder::new(table_name, vec![0.0_f32; dims], 1)
                .with_payload(true)
                .filter(filter);
        let response: SearchResponse = self.search_points(search_request).await?;
        let has_it = response.result.len() > 0;
        return Ok(has_it);
    }

    fn search_to_str(search: SearchResponse) -> Vec<String> {
        let vec = search.result;
        let mut result = Vec::with_capacity(vec.len());

        for value in &vec {
            let conf = &value.score;
            let hashmap = &value.payload;
            let string = hashmap.get("text").unwrap().as_str().unwrap();
            let full_str = format!("score: {}, text: {}", conf, string);
            result.push(full_str);
        }

        result
    }
}

// pub async fn dump(client: &Qdrant, table_name: &str) -> Result<()> {
//     let response = client
//         .scroll(ScrollPoints {
//             collection_name: table_name.to_owned(),
//             limit: Some(100), // how many points to fetch at once
//             with_payload: Some(WithPayloadSelector {
//                 selector_options: Some(SelectorOptions::Enable(true)), // include payloads
//             }),
//             with_vectors: Some(true.into()), // include vectors
//             ..Default::default()
//         })
//         .await?;

//     // Print all retrieved points
//     for point in response.result {
//         println!("{:?}", point);
//     }

//     Ok(())
// }

// TO DO, SEARCH IS NOT SORTED
