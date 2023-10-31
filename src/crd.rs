use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "unchessful.games",
    version = "v1",
    kind = "Engine",
    namespaced
)]
pub struct EngineV1Crd {
    pub url: String,
}
