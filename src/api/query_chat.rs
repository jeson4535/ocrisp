use crate::abstractions::AI;
use crate::db::DBTrait;
use anyhow::Result;
use qdrant_client::Qdrant;
use std::io::{self, BufRead, BufReader};

pub async fn run_query_chat() -> Result<()> {
    let _handle = Qdrant::run_db(true).ok();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let db = Some(Qdrant::init(None)?).unwrap();
    let ai = AI::new("http://localhost:11434/api/embed", "embeddinggemma", 768);
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        
        let s = crate::db::simple_search(&db, &ai, &line, None).await;
        println!("{}",s);
    }

    Ok(())
}
