mod cleanup;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use serde::Serialize;

use crate::app::AppContext;
use crate::error::CoreError;

const DAY_SECONDS: u64 = 24 * 60 * 60;
const GIB_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CachePolicy {
    payload_retention_secs: u64,
    source_retention_secs: u64,
    fixed_trigger_bytes: u64,
    filesystem_trigger_percent: u64,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            payload_retention_secs: 90 * DAY_SECONDS,
            source_retention_secs: 30 * DAY_SECONDS,
            fixed_trigger_bytes: 20 * GIB_BYTES,
            filesystem_trigger_percent: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct CachePolicyReport {
    pub payload_retention_days: u64,
    pub source_retention_days: u64,
    pub fixed_trigger_bytes: u64,
    pub filesystem_trigger_bytes: u64,
    pub effective_trigger_bytes: u64,
    pub usage_bytes: u64,
    pub package_usage_bytes: u64,
    pub source_usage_bytes: u64,
    pub package_entry_count: usize,
    pub source_entry_count: usize,
    pub needs_cleanup: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub(crate) struct CacheCleanupReport {
    pub usage_before_bytes: u64,
    pub usage_after_bytes: u64,
    pub effective_trigger_bytes: u64,
    pub deleted_entries: Vec<DeletedCacheEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DeletedCacheEntry {
    pub path: String,
    pub entry_kind: String,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone)]
struct UsageTotals {
    package_usage_bytes: u64,
    source_usage_bytes: u64,
    package_entry_count: usize,
    source_entry_count: usize,
    filesystem_trigger_bytes: u64,
}

#[derive(Debug, Clone)]
struct PrunableEntry {
    display_path: PathBuf,
    entry_kind: &'static str,
    bytes: u64,
    last_access_unix: u64,
    paths: Vec<PathBuf>,
}

impl AppContext {
    pub(crate) fn cache_policy_report(&self) -> Result<CachePolicyReport, CoreError> {
        self.database.bootstrap()?;
        cleanup::build_cache_policy_report(&self.database, CachePolicy::default())
    }

    pub(crate) fn reconcile_cache_policy(&self) -> Result<CacheCleanupReport, CoreError> {
        self.database.bootstrap()?;
        cleanup::reconcile_cache_policy(&self.database, CachePolicy::default())
    }
}

fn effective_trigger_bytes(usage: &UsageTotals, policy: CachePolicy) -> u64 {
    policy
        .fixed_trigger_bytes
        .min(usage.filesystem_trigger_bytes.max(1))
}

fn usage_bytes(usage: &UsageTotals) -> u64 {
    usage.package_usage_bytes + usage.source_usage_bytes
}

fn build_policy_report(usage: UsageTotals, policy: CachePolicy) -> CachePolicyReport {
    let effective_trigger_bytes = effective_trigger_bytes(&usage, policy);

    CachePolicyReport {
        payload_retention_days: policy.payload_retention_secs / DAY_SECONDS,
        source_retention_days: policy.source_retention_secs / DAY_SECONDS,
        fixed_trigger_bytes: policy.fixed_trigger_bytes,
        filesystem_trigger_bytes: usage.filesystem_trigger_bytes,
        effective_trigger_bytes,
        usage_bytes: usage_bytes(&usage),
        package_usage_bytes: usage.package_usage_bytes,
        source_usage_bytes: usage.source_usage_bytes,
        package_entry_count: usage.package_entry_count,
        source_entry_count: usage.source_entry_count,
        needs_cleanup: usage_bytes(&usage) > effective_trigger_bytes,
    }
}
