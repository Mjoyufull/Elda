use crate::app::{AppContext, ResolvedInstallTarget};
use crate::error::CoreError;
use elda_build::{BinaryCache, BinarySourceVerification};
use elda_recipe::{ScalarValue, SourceDefinition};
use elda_repo::{RemotePayloadTrust, RepoError, SyncedPackageRecord, TrustMode, list_caches};

impl AppContext {
    pub(crate) fn apply_remote_snapshot_metadata(
        &self,
        resolved: &mut ResolvedInstallTarget,
        package: &SyncedPackageRecord,
        payload_trust: &RemotePayloadTrust,
    ) -> Result<(), CoreError> {
        resolved.remote_name = Some(package.remote_name.clone());

        if !matches!(
            resolved.selected_source_kind.as_str(),
            "url_archive" | "github_release"
        ) {
            return Ok(());
        }

        if let Some(asset_url) = &package.asset_url {
            let sha256 = package.sha256.clone().ok_or_else(|| {
                CoreError::Repo(RepoError::Parse(format!(
                    "remote package `{}` is missing indexed `sha256` for binary asset metadata",
                    package.pkgname
                )))
            })?;
            resolved.recipe.package.source = remote_binary_source_definition(
                &resolved.recipe.package.source,
                asset_url,
                &sha256,
            );
        }

        resolved.binary_source_verification =
            remote_binary_source_verification(package, payload_trust)?;

        Ok(())
    }

    pub(crate) fn configured_binary_caches(&self) -> Result<Vec<BinaryCache>, CoreError> {
        let mut caches = list_caches(&self.database.layout().caches_dir)?
            .into_iter()
            .filter(|cache| cache.enabled)
            .map(|cache| BinaryCache {
                name: cache.name,
                base_url: cache.base_url,
                priority: cache.priority,
            })
            .collect::<Vec<_>>();
        caches.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.name.cmp(&right.name))
        });

        Ok(caches)
    }
}

fn remote_binary_source_definition(
    source: &SourceDefinition,
    asset_url: &str,
    sha256: &str,
) -> SourceDefinition {
    let mut fields = source.fields.clone();
    fields.insert("url".to_owned(), ScalarValue::String(asset_url.to_owned()));
    fields.insert("sha256".to_owned(), ScalarValue::String(sha256.to_owned()));

    SourceDefinition::single_lane("url_archive".to_owned(), fields)
}

fn remote_binary_source_verification(
    package: &SyncedPackageRecord,
    payload_trust: &RemotePayloadTrust,
) -> Result<Option<BinarySourceVerification>, CoreError> {
    if payload_trust.trust == TrustMode::Insecure {
        return Ok(None);
    }
    if !payload_trust.verified {
        return Err(CoreError::Repo(RepoError::Trust(format!(
            "remote `{}` is not verified and cannot provide signed payloads",
            package.remote_name
        ))));
    }

    Ok(Some(BinarySourceVerification {
        remote_name: package.remote_name.clone(),
        payload_signature: package.payload_sig.clone(),
        trusted_public_keys: payload_trust
            .trusted_public_keys
            .iter()
            .map(|trusted| trusted.public_key.clone())
            .collect(),
    }))
}
