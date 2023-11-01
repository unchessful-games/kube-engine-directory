mod crd;
mod engine_tracker;
mod server;
use std::convert::Infallible;

#[tokio::main]
#[cfg(not(feature = "print-crd"))]
async fn main() -> anyhow::Result<Infallible> {
    use crate::server::run_server;
    use futures_util::stream::StreamExt;
    use kube::{
        runtime::{reflector, watcher::Config},
        Api, Client,
    };
    use tokio::sync::mpsc;

    use crate::{crd::Engine, engine_tracker::EngineTrackerTask};
    use engine_tracker::engine_tracker;

    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let engine_crds: Api<Engine> = Api::namespaced(client, "unchessful");
    let lp = Config::default();
    let (reader, writer) = reflector::store();
    let mut rf = Box::pin(reflector(writer, kube::runtime::watcher(engine_crds, lp)));

    let (tx, rx) = mpsc::channel(100);
    tokio::spawn(engine_tracker(reader, rx, tx.clone()));
    tokio::spawn(run_server(tx.clone()));

    while let Some(v) = rf.next().await {
        let v = v?;
        //println!("{v:?}");
        match v {
            kube::runtime::watcher::Event::Applied(v) => tx
                .send(EngineTrackerTask::CreateUrl(v.spec.url))
                .await
                .unwrap(),
            kube::runtime::watcher::Event::Deleted(v) => tx
                .send(EngineTrackerTask::DeleteUrl(v.spec.url))
                .await
                .unwrap(),
            kube::runtime::watcher::Event::Restarted(v) => {
                for item in v {
                    tx.send(EngineTrackerTask::CreateUrl(item.spec.url))
                        .await
                        .unwrap()
                }
            }
        }
    }

    unreachable!()
}

#[cfg(feature = "print-crd")]
fn main() {
    use crd::Engine;
    use kube::CustomResourceExt;

    println!("{}", serde_yaml::to_string(&Engine::crd()).unwrap());
}
