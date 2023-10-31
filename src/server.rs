use std::collections::HashMap;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::engine_tracker::EngineTrackerTask;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EngineDirectory {
    pub engines: HashMap<String, EngineEntry>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "status")]
pub enum EngineEntry {
    /// The engine does not respond at the moment.
    Offline,
    /// The engine is available, and this is the name and description.
    Online { id: String, description: String },
}

pub async fn run_server(task_sender: mpsc::Sender<EngineTrackerTask>) -> ! {
    let router = axum::Router::new()
        .route("/", axum::routing::get(fetch_directory))
        .with_state(task_sender);
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
    unreachable!();
}

async fn fetch_directory(
    State(sender): State<mpsc::Sender<EngineTrackerTask>>,
) -> Json<EngineDirectory> {
    let (tx, rx) = oneshot::channel();

    sender
        .send(EngineTrackerTask::FetchEngines(tx))
        .await
        .unwrap();

    let dir = rx.await.unwrap();

    Json(dir)
}
