use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use engine_trait::server_types::AnyEngineInfo;
use kube::runtime::reflector::Store;
use rand::Rng;
use tokio::sync::{mpsc, oneshot};

use crate::{
    crd::Engine,
    server::{EngineDirectory, EngineEntry},
};

pub async fn engine_tracker(
    store: Store<Engine>,
    mut tasks: mpsc::Receiver<EngineTrackerTask>,
    task_sender: mpsc::Sender<EngineTrackerTask>,
) -> ! {
    let mut status = HashMap::new();
    // Spawn a task to send us a Reconcile message once every minute.
    async fn reconciliation_loop(task_sender: mpsc::Sender<EngineTrackerTask>) -> ! {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            task_sender
                .send(EngineTrackerTask::Reconcile)
                .await
                .unwrap();
        }
    }
    tokio::spawn(reconciliation_loop(task_sender.clone()));

    loop {
        let task = tasks.recv().await.unwrap();
        tracing::debug!("Received task: {task:?}");
        match task {
            EngineTrackerTask::FetchEngines(sender) => {
                sender
                    .send(EngineDirectory {
                        engines: status.clone(),
                    })
                    .unwrap();
            }
            EngineTrackerTask::CreateUrl(url) => {
                status.entry(url.clone()).or_insert(EngineEntry::Offline);
                tokio::spawn(check_engine(url, task_sender.clone()));
            }
            EngineTrackerTask::DeleteUrl(url) => {
                status.remove(&url);
            }
            EngineTrackerTask::Reconcile => {
                // Before we've seen any items in the state, we would delete everything.
                let mut to_delete: HashSet<_> = status.keys().cloned().collect();
                let mut to_add: HashSet<_> = HashSet::new();

                // As we're seeing new engines, we're deciding not to delete them,
                // or add them if they aren't there.
                for engine in store.state() {
                    to_delete.remove(&engine.spec.url);
                    to_add.insert(engine.spec.url.clone());
                }

                // All the items that were in our status, but not in the kube store: delete them.
                for item in to_delete {
                    status.remove(&item);
                }

                // All the items that are in the kube store: if missing, add an offline entry.
                for item in to_add {
                    status.entry(item).or_insert(EngineEntry::Offline);
                }

                // Spawn tasks to check each of the existing engines.
                for url in status.keys().cloned() {
                    tokio::spawn(check_engine(url, task_sender.clone()));
                }
            }
            EngineTrackerTask::ObserveStatus(url, current_status) => {
                status.insert(url, current_status);
            }
        }
    }
}

async fn check_engine(url: String, task_sender: mpsc::Sender<EngineTrackerTask>) {
    // Wait a random amount of time, up to 30 seconds, in order to manage the load on the engines.
    let seconds_to_wait = rand::thread_rng().gen_range(0.0..30.0);
    tokio::time::sleep(Duration::from_secs_f64(seconds_to_wait)).await;

    match reqwest::get(&url).await {
        Ok(resp) => match resp.error_for_status() {
            Ok(resp) => {
                let info: AnyEngineInfo = match resp.json().await {
                    Ok(v) => v,
                    Err(why) => {
                        tracing::warn!("Could not fetch URL {url}: {why}");
                        task_sender
                            .send(EngineTrackerTask::ObserveStatus(url, EngineEntry::Offline))
                            .await
                            .unwrap();
                        return;
                    }
                };

                tracing::debug!("Fetched {url} OK!");
                task_sender
                    .send(EngineTrackerTask::ObserveStatus(
                        url,
                        EngineEntry::Online {
                            id: info.id,
                            description: info.description,
                        },
                    ))
                    .await
                    .unwrap();
            }
            Err(why) => {
                tracing::warn!("Could not fetch URL {url}: {why}");
                task_sender
                    .send(EngineTrackerTask::ObserveStatus(url, EngineEntry::Offline))
                    .await
                    .unwrap();
                return;
            }
        },
        Err(why) => {
            tracing::warn!("Could not fetch URL {url}: {why}");
            task_sender
                .send(EngineTrackerTask::ObserveStatus(url, EngineEntry::Offline))
                .await
                .unwrap();
            return;
        }
    }
}

#[derive(Debug)]
pub enum EngineTrackerTask {
    /// Fetch a copy of the current engine states.
    FetchEngines(oneshot::Sender<EngineDirectory>),

    /// Observe a new URL being added.
    CreateUrl(String),

    /// Observe an existing URL being deleted.
    DeleteUrl(String),

    /// Trigger a reconciliation: check the storage,
    /// and then try rescanning the engine status.
    Reconcile,

    /// Observe that an engine has a particular status.
    ObserveStatus(String, EngineEntry),
}
