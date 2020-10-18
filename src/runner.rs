use anyhow::anyhow;
use chrono::{DateTime, Utc};
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use k8s_openapi::api::core::v1::Secret;
use log;
use std::collections::HashMap;

use crate::crd::{VaultStoreRule, VaultStoreRuleRollout, VaultStoreRuleStatus};
use crate::error::Error::{RuleExecutionFailed, UnsupportedRolloutKind};
use crate::vault_client;

pub struct Runner {
    kube: kube::Client,
    kube_crd: kube::Api<VaultStoreRule>,
    vault_client: vault_client::Client,
    now: DateTime<Utc>,
}

impl Runner {
    pub fn new(
        kube_client: kube::Client,
        vault_client: vault_client::Client,
        namespace: Option<String>,
    ) -> Self {
        let kube_crd: kube::Api<VaultStoreRule> = if let Some(ns) = namespace.clone() {
            kube::Api::namespaced(kube_client.clone(), &ns)
        } else {
            kube::Api::all(kube_client.clone())
        };
        return Runner {
            kube: kube_client,
            kube_crd,
            vault_client,
            now: chrono::Utc::now(),
        };
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        log::info!("Finding rules...");
        let mut list = self
            .kube_crd
            .list(&kube::api::ListParams::default())
            .await?
            .into_iter();
        let mut failed = false;
        while let Some(rule) = list.next() {
            log::info!("");
            log::info!(
                "Rule: {}/{}",
                rule.metadata
                    .namespace
                    .as_ref()
                    .unwrap_or(&"default".to_string()),
                rule.metadata.name.as_ref().unwrap(),
            );
            let rule_result = self.run_rule(&rule).await;
            if rule_result.is_err() {
                log::error!("!!!! Failed: {}", rule_result.err().unwrap());
                failed = true;
            }
        }
        if failed {
            Err(anyhow!(RuleExecutionFailed))
        } else {
            log::info!("Successfully completed");
            Ok(())
        }
    }

    async fn run_rule(&self, rule: &VaultStoreRule) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("===> Checking status");
        let mut status = if let Some(status) = rule.status.clone() {
            log::info!("     Status: {:?}", &status);
            status
        } else {
            log::info!("     Status: (new)");
            VaultStoreRuleStatus::default()
        };
        let lease_not_exists = status.lease_id.is_none();
        let orig_expires_at = status.expires_at;
        let orphaned_lease_id = status.next_lease_id.clone();
        let last_successful_run_at = status.last_successful_run_at;

        // record this run
        status.last_run_started_at = Some(self.now.clone());
        self.patch_status(&rule, &status).await?;

        // discard an expired lease when present
        let needs_discard = orig_expires_at
            .map(|date| date <= self.now)
            .unwrap_or(false);
        if needs_discard {
            log::warn!(
                "   * Discarding the current lease={:?} as it seems to be expired",
                &status.lease_id
            );
        }

        let needs_rotate = lease_not_exists
            || needs_discard
            || check_request_annotation(
                last_successful_run_at.as_ref(),
                rule.metadata
                    .annotations
                    .as_ref()
                    .and_then(|a| a.get("vault2kube.sorah.jp/rotateRequestedAt")),
            )
            || is_time_after_deadline(&self.now, &orig_expires_at, rule.spec.rotate_before_seconds);

        let needs_renew = check_request_annotation(
            last_successful_run_at.as_ref(),
            rule.metadata
                .annotations
                .as_ref()
                .and_then(|a| a.get("vault2kube.sorah.jp/renewRequestedAt")),
        ) || is_time_after_deadline(
            &self.now,
            &orig_expires_at,
            rule.spec.renew_before_seconds,
        );

        // try revoke
        if status.last_lease_id.is_some()
            && (is_time_after_deadline(
                &self.now,
                &status.rotated_at,
                rule.spec.revoke_after_seconds.and_then(|s| Some(s * -1)),
            ) || needs_rotate)
        {
            self.revoke_last(&mut status).await?;
        }

        // renew
        if !needs_discard && !needs_rotate && needs_renew {
            self.renew(&mut status).await?;
        }

        // rotate
        if needs_rotate {
            let lease = self
                .rotate(
                    &rule.spec.source_path,
                    rule.spec.parameters.as_ref(),
                    &mut status,
                )
                .await?;

            // Save the fresh lease_id as soon as possible, to easily revoke them later in case of any failure
            // may occur in the same run.
            self.patch_status_next_lease_id(&rule, Some(lease.lease_id.to_owned().as_ref()))
                .await?;

            self.update_secret(&rule, lease).await?;

            self.patch_status_next_lease_id(&rule, None).await?;
        }

        // rollout
        if needs_rotate {
            if let Some(rollout_resources) = rule.spec.rollout_restarts.clone() {
                let namespace = rule
                    .metadata
                    .namespace
                    .clone()
                    .unwrap_or("default".to_string());
                self.rollout(&namespace, &rollout_resources).await?;
            }
        }

        // revoke unused secret due to possible failure
        if let Some(lease_id) = orphaned_lease_id {
            log::warn!("   * Revoking orphaned lease_id={:?}", lease_id);
            self.vault_client.revoke(&lease_id).await?;
        }

        // update status
        status.next_lease_id = None;
        status.last_successful_run_at = Some(self.now.clone());
        self.patch_status(&rule, &status).await?;
        log::info!("===> Complete");
        log::info!("   * Status: {:?}", status);

        Ok(())
    }

    async fn patch_status(
        &self,
        rule: &VaultStoreRule,
        status: &VaultStoreRuleStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("   > Updating status: {:?}", status);

        let kube_crd: kube::Api<VaultStoreRule> = kube::Api::namespaced(
            self.kube.clone(),
            &rule
                .metadata
                .namespace
                .as_ref()
                .unwrap_or(&"default".to_string()),
        );

        let patch = serde_yaml::to_vec(&serde_json::json!({
            "apiVersion": "vault2kube.sorah.jp/v1",
            "kind": "VaultStoreRule",
            "status": status
        }))?;

        kube_crd
            .patch_status(
                rule.metadata.name.as_ref().ok_or("name is missing")?,
                &kube::api::PatchParams::apply("vault2kube.sorah.jp").force(),
                patch,
            )
            .await?;
        Ok(())
    }

    // Adhoc function to send only "next_lease_id" field in a patch
    async fn patch_status_next_lease_id(
        &self,
        rule: &VaultStoreRule,
        next_lease_id: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("   > Updating status.next_lease_id: {:?}", next_lease_id);

        let kube_crd: kube::Api<VaultStoreRule> = kube::Api::namespaced(
            self.kube.clone(),
            &rule
                .metadata
                .namespace
                .as_ref()
                .unwrap_or(&"default".to_string()),
        );

        let patch = serde_yaml::to_vec(&serde_json::json!({
            "apiVersion": "vault2kube.sorah.jp/v1",
            "kind": "VaultStoreRule",
            "status": {"nextLeaseId": next_lease_id}
        }))?;

        // Intentionally force
        // https://v1-17.docs.kubernetes.io/docs/reference/using-api/api-concepts/#conflicts
        // https://github.com/cybozu-go/cke/issues/311
        kube_crd
            .patch_status(
                rule.metadata.name.as_ref().ok_or("name is missing")?,
                &kube::api::PatchParams::apply("vault2kube.sorah.jp").force(),
                patch,
            )
            .await?;
        Ok(())
    }

    async fn revoke_last(
        &self,
        status: &mut VaultStoreRuleStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::warn!("===> Revoking the last lease={:?}", &status.last_lease_id);
        self.vault_client
            .revoke(status.last_lease_id.as_ref().unwrap())
            .await?;
        status.last_lease_id = None;
        Ok(())
    }

    async fn renew(
        &self,
        status: &mut VaultStoreRuleStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("===> Renewing the current lease={:?}", &status.lease_id);
        let original_ttl = status.ttl.unwrap();
        let lease = self
            .vault_client
            .renew(status.lease_id.as_ref().unwrap())
            .await?;
        status.ttl = Some(lease.lease_duration);
        status.expires_at =
            Some(Utc::now() + chrono::Duration::seconds(status.ttl.unwrap() as i64));

        log::info!(
            "   * renewed: ttl={:?}, expires_at={:?}",
            &status.ttl,
            &status.expires_at
        );

        // In case ttl is capped to max_ttl
        if lease.lease_duration < original_ttl {
            log::info!("   * Renewed, but seems to be capped to max_ttl. Will rotate. (original_ttl={:?}, ttl={:?})", &original_ttl, &lease.lease_duration);
            status.lease_id = None;
            status.ttl = None;
            status.expires_at = None;
            status.last_lease_id = status.lease_id.clone();
            status.rotated_at = Some(self.now.clone());
        }
        Ok(())
    }

    async fn rotate(
        &self,
        source_path: &str,
        parameters: Option<&serde_json::Map<String, serde_json::Value>>,
        status: &mut VaultStoreRuleStatus,
    ) -> Result<vault_client::LeaseResponse, Box<dyn std::error::Error>> {
        log::info!("===> Acquiring a new Vault lease at path={:?}", source_path);
        let lease = match parameters {
            Some(params) => {
                log::info!("   * params: {:}", serde_json::json! {params});
                self.vault_client.write(&source_path, &params).await?
            }
            None => self.vault_client.read(&source_path).await?,
        };
        if let Some(original_lease_id) = status.lease_id.clone() {
            status.last_lease_id = Some(original_lease_id);
            status.rotated_at = Some(self.now);
        }
        status.lease_id = Some(lease.lease_id.clone());
        status.ttl = Some(lease.lease_duration);
        status.expires_at =
            Some(Utc::now() + chrono::Duration::seconds(status.ttl.unwrap() as i64));
        log::info!(
            "   * Lease acquired: lease_id={:?}, ttl={:?}, expires_at={:?}",
            &status.lease_id,
            &status.ttl,
            &status.expires_at
        );

        Ok(lease)
    }

    async fn update_secret(
        &self,
        rule: &VaultStoreRule,
        lease: vault_client::LeaseResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("===> Applying secret: {}", &rule.spec.destination_name);
        let default_ns = "default".to_string();
        let namespace = &rule.metadata.namespace.as_ref().unwrap_or(&default_ns);

        let hb = handlebars::Handlebars::new();
        let secrets: kube::Api<Secret> = kube::Api::namespaced(self.kube.clone(), namespace);

        let mut string_data: HashMap<String, String> = HashMap::new();
        let mut iter = rule.spec.templates.iter();

        while let Some(tmpl) = iter.next() {
            log::info!("   * key={:?}, template={:?}", &tmpl.key, &tmpl.template,);
            let value = hb.render_template(&tmpl.template, &lease.data)?;
            string_data.insert(tmpl.key.to_owned(), value);
        }

        let patch = serde_json::json!({
            "apiVersion": "v1",
            "kind": "Secret",
            "metadata": {
                "name": &rule.spec.destination_name,
                "namespace": &namespace,
                "labels": {
                    "kubernetes.io/managed-by": "vault2kube.sorah.jp",
                    "vault2kube.sorah.jp/rule": &rule.metadata.name
                },
            },
            "stringData": string_data,
        });

        // Intentionally force
        // https://v1-17.docs.kubernetes.io/docs/reference/using-api/api-concepts/#conflicts
        // https://github.com/cybozu-go/cke/issues/311
        let patch_response = secrets
            .patch(
                &rule.spec.destination_name,
                &kube::api::PatchParams::apply("vault2kube.sorah.jp").force(),
                serde_yaml::to_vec(&patch)?,
            )
            .await;
        // Retry as a create request when a resource does not exist yet
        if let Err(e) = patch_response {
            match &e {
                kube::error::Error::Api(ae) => {
                    if ae.code != 404 {
                        return Err(Box::new(e));
                    }
                    log::debug!("     (got 404, creating instead of patch)");
                    let patch_json: Secret = serde_json::from_value(patch)?;
                    secrets
                        .create(&kube::api::PostParams::default(), &patch_json)
                        .await?;
                }
                _ => return Err(Box::new(e)),
            }
        }
        Ok(())
    }

    async fn rollout(
        &self,
        namespace: &String,
        resources: &Vec<VaultStoreRuleRollout>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("===> Rolling out");
        let mut iter = resources.iter();
        while let Some(rollout) = iter.next() {
            let kind = rollout.kind.as_str();
            log::info!("   * restart: {}/{}/{}", namespace, &kind, &rollout.name);
            match kind {
                "Deployment" => {
                    self.rollout_single::<Deployment>(namespace, &kind, &rollout.name)
                        .await
                }
                "DaemonSet" => {
                    self.rollout_single::<DaemonSet>(namespace, &kind, &rollout.name)
                        .await
                }
                "StatefulSet" => {
                    self.rollout_single::<StatefulSet>(namespace, &kind, &rollout.name)
                        .await
                }
                _ => return Err(Box::new(UnsupportedRolloutKind)),
            }?;
        }
        Ok(())
    }

    async fn rollout_single<
        T: k8s_openapi::Resource + Clone + serde::de::DeserializeOwned + kube::api::Meta,
    >(
        &self,
        namespace: &String,
        kind: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client: kube::Api<T> = kube::Api::namespaced(self.kube.clone(), namespace);
        let patch = serde_yaml::to_vec(&serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": kind,
            "spec": {
                "template": {
                    "metadata": {
                        "annotations": {
                            "vault2kube.sorah.jp/restartedAt": self.now,
                        },
                    },
                },
            },
        }))?;

        // Intentionally force
        // https://v1-17.docs.kubernetes.io/docs/reference/using-api/api-concepts/#conflicts
        // https://github.com/cybozu-go/cke/issues/311
        client
            .patch(
                name,
                &kube::api::PatchParams::apply("vault2kube.sorah.jp").force(),
                patch,
            )
            .await?;
        Ok(())
    }
}

fn check_request_annotation(now: Option<&DateTime<Utc>>, iso8601str: Option<&String>) -> bool {
    if iso8601str.is_none() {
        return false;
    }
    if now.is_none() {
        return true;
    }
    if let Ok(requested_at) = DateTime::parse_from_rfc3339(&iso8601str.unwrap()) {
        return now.unwrap() < &requested_at;
    }
    false
}

fn is_time_after_deadline(
    now: &DateTime<Utc>,
    target: &Option<DateTime<Utc>>,
    deadline_sec: Option<i32>,
) -> bool {
    if target.is_none() || deadline_sec.is_none() {
        return false;
    }
    let offset = chrono::Duration::seconds(deadline_sec.unwrap() as i64);
    let deadline = target.unwrap() - offset;
    &deadline <= now
}
