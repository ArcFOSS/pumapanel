//! PaperMC version catalogue.
//!
//! When creating a server the panel must know which PaperMC versions exist and
//! what minimum Java each one needs, because that minimum picks the container
//! image tag. The browser never talks to PaperMC: this module proxies the API
//! and caches the result in memory, so upstream is hit at most once per TTL.
//!
//! PaperMC's older `api.papermc.io/v2` endpoints now return `410 Gone`; the
//! current API is `fill.papermc.io/v3`, whose version list already carries both
//! the support status and the minimum Java, so a single request is enough.

use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::error::{ApiError, FieldError};

/// PaperMC asks API clients to identify themselves.
const USER_AGENT: &str = concat!("pumapanel/", env!("CARGO_PKG_VERSION"));

/// One selectable PaperMC version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperVersion {
    /// The version identifier, e.g. `1.21.8`.
    pub version: String,
    /// PaperMC's own support status (`SUPPORTED`/`UNSUPPORTED`). Passed through
    /// verbatim so the UI can label it without the panel inventing a policy.
    pub support: String,
    /// Minimum Java version, used to pick the container image tag.
    pub java_min: Option<u8>,
}

/// The version list the frontend receives.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionsResponse {
    pub versions: Vec<PaperVersion>,
    /// True when PaperMC could not be reached and a cached copy was served, so
    /// the UI can say the list may be out of date.
    pub stale: bool,
}

/// Proxies and caches PaperMC's version catalogue.
#[derive(Clone)]
pub struct PaperClient {
    http: reqwest::Client,
    base_url: reqwest::Url,
    ttl: Duration,
    cache: Arc<Mutex<Option<Cached>>>,
}

struct Cached {
    versions: Vec<PaperVersion>,
    fetched: Instant,
}

impl PaperClient {
    pub fn new(config: &Config) -> Result<Self, ApiError> {
        let mut base_url = reqwest::Url::parse(&config.paper_api_url).map_err(|error| {
            ApiError::Unavailable(format!("PAPER_API_URL is not a valid URL: {error}"))
        })?;
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path());
            base_url.set_path(&path);
        }

        let http = reqwest::Client::builder()
            .timeout(config.paper_timeout)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| {
                ApiError::Upstream(format!("Could not build the PaperMC HTTP client: {error}"))
            })?;

        Ok(Self {
            http,
            base_url,
            ttl: config.paper_cache_ttl,
            cache: Arc::new(Mutex::new(None)),
        })
    }

    /// The catalogue, served from cache while it is fresh.
    ///
    /// The cache lock is held across a refresh so concurrent requests collapse
    /// into a single upstream call. If PaperMC is unavailable but a cached copy
    /// exists, the stale copy is served; only a cold cache fails.
    pub async fn versions(&self) -> Result<VersionsResponse, ApiError> {
        let mut cache = self.cache.lock().await;

        if let Some(cached) = cache.as_ref()
            && cached.fetched.elapsed() < self.ttl
        {
            return Ok(VersionsResponse {
                versions: cached.versions.clone(),
                stale: false,
            });
        }

        match self.fetch().await {
            Ok(versions) => {
                *cache = Some(Cached {
                    versions: versions.clone(),
                    fetched: Instant::now(),
                });
                Ok(VersionsResponse {
                    versions,
                    stale: false,
                })
            }
            Err(error) => match cache.as_ref() {
                Some(cached) => Ok(VersionsResponse {
                    versions: cached.versions.clone(),
                    stale: true,
                }),
                None => Err(error),
            },
        }
    }

    /// Looks up one version, producing the field error the API returns when the
    /// catalogue does not offer it.
    pub async fn find(&self, version: &str) -> Result<PaperVersion, ApiError> {
        self.versions()
            .await?
            .versions
            .into_iter()
            .find(|candidate| candidate.version == version)
            .ok_or_else(|| {
                ApiError::Validation(vec![FieldError {
                    field: "version".to_string(),
                    message: format!("`{version}` is not an available PaperMC version"),
                }])
            })
    }

    async fn fetch(&self) -> Result<Vec<PaperVersion>, ApiError> {
        let url = self
            .base_url
            .join("projects/paper/versions")
            .map_err(|error| ApiError::Unavailable(format!("Invalid PaperMC URL: {error}")))?;

        let response = self.http.get(url).send().await.map_err(|error| {
            ApiError::Upstream(format!("Could not reach the PaperMC API: {error}"))
        })?;

        if !response.status().is_success() {
            return Err(ApiError::Upstream(format!(
                "The PaperMC API responded with {}",
                response.status()
            )));
        }

        let body = response.text().await.map_err(|error| {
            ApiError::Upstream(format!("Could not read the PaperMC response: {error}"))
        })?;

        parse_versions(&body)
    }
}

/// The subset of the Fill v3 payload the panel needs. Everything else (build
/// numbers, recommended JVM flags) is ignored rather than forwarded.
mod fill {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Payload {
        pub versions: Vec<Entry>,
    }

    #[derive(Deserialize)]
    pub struct Entry {
        pub version: Version,
    }

    #[derive(Deserialize)]
    pub struct Version {
        pub id: String,
        #[serde(default)]
        pub support: Option<Support>,
        #[serde(default)]
        pub java: Option<Java>,
    }

    #[derive(Deserialize)]
    pub struct Support {
        pub status: String,
    }

    #[derive(Deserialize)]
    pub struct Java {
        pub version: JavaVersion,
    }

    #[derive(Deserialize)]
    pub struct JavaVersion {
        #[serde(default)]
        pub minimum: Option<u8>,
    }
}

/// Reads Fill's version list, dropping pre-releases and ordering newest first.
///
/// Pre-releases (`1.21.11-rc1`, `26.3-rc-3`, ...) are filtered out: they are not
/// what a panel should offer by default. Release identifiers are purely dotted
/// numbers, so the presence of a `-` is exactly what marks a pre-release.
///
/// `SUPPORTED`/`UNSUPPORTED` is deliberately *not* used as a filter. PaperMC
/// marks only its newest line as supported, so filtering would leave the picker
/// with a couple of choices and hide the versions people actually run; the
/// status is returned for the UI to show as a badge instead.
pub fn parse_versions(body: &str) -> Result<Vec<PaperVersion>, ApiError> {
    let payload: fill::Payload = serde_json::from_str(body).map_err(|error| {
        ApiError::Upstream(format!("Could not read the PaperMC version list: {error}"))
    })?;

    let mut versions: Vec<PaperVersion> = payload
        .versions
        .into_iter()
        .filter_map(|entry| {
            let version = entry.version;
            if version.id.contains('-') {
                return None;
            }

            Some(PaperVersion {
                version: version.id,
                support: version
                    .support
                    .map(|support| support.status)
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                java_min: version.java.and_then(|java| java.version.minimum),
            })
        })
        .collect();

    versions.sort_by(|left, right| compare_versions(&right.version, &left.version));

    Ok(versions)
}

/// Orders dotted numeric versions (`1.21.8` after `1.20.6`).
///
/// Comparing segment by segment numerically — rather than comparing the strings
/// — is what keeps `1.9` above `1.10` and `1.21.9` below `1.21.11`. Missing
/// trailing segments count as zero, so `1.21` and `1.21.0` are the same release.
fn compare_versions(left: &str, right: &str) -> Ordering {
    let left = numeric_parts(left);
    let right = numeric_parts(right);

    for index in 0..left.len().max(right.len()) {
        let ordering = left
            .get(index)
            .copied()
            .unwrap_or(0)
            .cmp(&right.get(index).copied().unwrap_or(0));
        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    Ordering::Equal
}

fn numeric_parts(version: &str) -> Vec<u32> {
    version
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Modelled on a real response: a supported release, a pre-release, two
    /// multi-digit pairs, a version with no `java` block, and an old release
    /// with a Java minimum the panel does not offer a tag for.
    const PAYLOAD: &str = r#"{
        "project": { "id": "paper", "name": "Paper" },
        "versions": [
            { "version": { "id": "26.3", "support": { "status": "SUPPORTED" },
                "java": { "version": { "minimum": 25 } } }, "builds": [3, 4] },
            { "version": { "id": "26.3-rc-3", "support": { "status": "UNSUPPORTED" },
                "java": { "version": { "minimum": 25 } } }, "builds": [1] },
            { "version": { "id": "1.21.11", "support": { "status": "UNSUPPORTED" },
                "java": { "version": { "minimum": 21 } } }, "builds": [60] },
            { "version": { "id": "1.21.9", "support": { "status": "UNSUPPORTED" },
                "java": { "version": { "minimum": 21 } } }, "builds": [1] },
            { "version": { "id": "1.21", "support": { "status": "UNSUPPORTED" } }, "builds": [1] },
            { "version": { "id": "1.20.6", "support": { "status": "UNSUPPORTED" },
                "java": { "version": { "minimum": 21 } } }, "builds": [1] },
            { "version": { "id": "1.10", "support": { "status": "UNSUPPORTED" },
                "java": { "version": { "minimum": 8 } } }, "builds": [1] },
            { "version": { "id": "1.9", "support": { "status": "UNSUPPORTED" },
                "java": { "version": { "minimum": 8 } } }, "builds": [1] },
            { "version": { "id": "1.7.10", "support": { "status": "UNSUPPORTED" },
                "java": { "version": { "minimum": 6 } } }, "builds": [1] }
        ]
    }"#;

    fn ids(versions: &[PaperVersion]) -> Vec<&str> {
        versions
            .iter()
            .map(|version| version.version.as_str())
            .collect()
    }

    #[test]
    fn drops_pre_releases_and_orders_newest_first() {
        let versions = parse_versions(PAYLOAD).expect("parsed");

        assert_eq!(
            ids(&versions),
            vec![
                "26.3", "1.21.11", "1.21.9", "1.21", "1.20.6", "1.10", "1.9", "1.7.10"
            ]
        );
    }

    #[test]
    fn carries_support_status_and_minimum_java() {
        let versions = parse_versions(PAYLOAD).expect("parsed");

        let newest = &versions[0];
        assert_eq!(newest.version, "26.3");
        assert_eq!(newest.support, "SUPPORTED");
        assert_eq!(newest.java_min, Some(25));

        let oldest = versions.last().expect("a version");
        assert_eq!(oldest.version, "1.7.10");
        assert_eq!(oldest.java_min, Some(6));
    }

    #[test]
    fn tolerates_a_version_without_java_or_support_details() {
        let body = r#"{ "versions": [ { "version": { "id": "1.21" } } ] }"#;

        let versions = parse_versions(body).expect("parsed");

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].java_min, None);
        assert_eq!(versions[0].support, "UNKNOWN");
    }

    #[test]
    fn compares_versions_numerically_not_alphabetically() {
        assert_eq!(compare_versions("1.10", "1.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.21.11", "1.21.9"), Ordering::Greater);
        assert_eq!(compare_versions("26.3", "1.21.11"), Ordering::Greater);
        assert_eq!(compare_versions("1.21", "1.21.0"), Ordering::Equal);
    }

    #[test]
    fn rejects_unreadable_payloads() {
        assert!(parse_versions("not json").is_err());
        assert!(parse_versions("{}").is_err());
    }
}
