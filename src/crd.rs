use chrono::{DateTime, Utc};
use kube_derive::CustomResource;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, Default)]
#[kube(group = "vault2kube.sorah.jp", version = "v1", namespaced)]
#[kube(status = "VaultStoreRuleStatus")]
#[serde(rename_all = "camelCase")]
pub struct VaultStoreRuleSpec {
    pub source_path: String,
    pub destination_name: String,
    pub templates: Vec<VaultStoreRuleTemplate>,
    pub rollout_restarts: Option<Vec<VaultStoreRuleRollout>>,
    pub renew_before_seconds: Option<i32>,
    pub rotate_before_seconds: Option<i32>,
    pub revoke_after_seconds: Option<i32>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct VaultStoreRuleTemplate {
    pub key: String,
    pub template: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct VaultStoreRuleRollout {
    pub kind: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VaultStoreRuleStatus {
    pub lease_id: Option<String>,
    pub ttl: Option<u32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub next_lease_id: Option<String>,
    pub last_lease_id: Option<String>,
    pub rotated_at: Option<DateTime<Utc>>,
    pub last_run_started_at: Option<DateTime<Utc>>,
    pub last_successful_run_at: Option<DateTime<Utc>>,
}
