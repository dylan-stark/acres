//! [`Catalog`], the shared client for the AIC API (ADR 0016).

mod cache;
mod limiter;
mod shared;

use std::{fmt::Display, path::PathBuf, sync::Arc, time::Duration};

use bytes::Bytes;
use reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderValue, RETRY_AFTER},
};
use serde::de::DeserializeOwned;
use tokio::time::Instant;

pub(crate) use cache::Cache;
pub use limiter::Lane;
use limiter::Limiter;
use shared::SharedBucket;

use crate::{
    ArtworkType, ConfigError, Department, Error, Item, Page, ParseError, RequestError,
    ResponseError,
    artworks::{
        Artwork, ArtworkSummary,
        request::{artworks, search},
    },
    config::{Config, DEFAULT_BASE_URI, get_acres_cache_dir, get_acres_state_dir},
};

/// The file in the state directory that holds the shared rate limit.
const RATE_LIMIT_FILE: &str = "rate-limit";

/// The page size for [`Catalog::departments`] and [`Catalog::artwork_types`]: the API's largest,
/// well above the 16 departments and 45 types it has.
pub const FACETS_LIMIT: u32 = 100;

/// What kind of resource a request fetches, which sets its queue and how long it stays cached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Resource {
    /// A search or listing page. Cached for 24 hours, since results change as the collection does.
    Page,
    /// One record, such as an artwork or its manifest. Cached for 7 days, since fields such as
    /// `is_on_view` change.
    Record,
    /// An image. Cached for good: the image for an `image_id` does not change.
    Image,
}

impl Resource {
    /// How long a cached response stays fresh. `None` is forever.
    pub fn lifetime(self) -> Option<Duration> {
        const HOUR: Duration = Duration::from_secs(60 * 60);
        match self {
            Resource::Page => Some(24 * HOUR),
            Resource::Record => Some(7 * 24 * HOUR),
            Resource::Image => None,
        }
    }

    /// The queue it waits in.
    pub fn lane(self) -> Lane {
        match self {
            Resource::Page | Resource::Record => Lane::Metadata,
            Resource::Image => Lane::Image,
        }
    }
}

/// A response, and where its time went.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Fetched<T> {
    /// The response.
    pub value: T,
    /// Whether it came from the cache, so nothing was sent.
    pub from_cache: bool,
    /// Time spent waiting in the queue, including any back-off between retries.
    pub wait: Duration,
    /// Time spent on the network: sending, then reading the body.
    pub fetch: Duration,
}

impl<T> Fetched<T> {
    /// Applies `f` to the value, keeping the timings.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Fetched<U> {
        Fetched {
            value: f(self.value),
            from_cache: self.from_cache,
            wait: self.wait,
            fetch: self.fetch,
        }
    }
}

/// A client for the AIC API that rate-limits, retries, caches, and identifies itself.
///
/// Build one and share it: clones are cheap and share one connection pool, one rate limit and one
/// cache. Every request waits for a token (60 a minute by default, the API's anonymous limit).
/// Dropping a request's future cancels it, even while it waits.
///
/// ```no_run
/// # async fn run() -> Result<(), acres::Error> {
/// use acres::{Catalog, artworks::{ArtworkSummary, request::search}};
///
/// let catalog = Catalog::builder().build()?;
/// let request = search::Request::builder()
///     .base_uri(catalog.base_uri())
///     .q(Some("monet".into()))
///     .fields(Some(ArtworkSummary::FIELDS.iter().map(|f| f.to_string()).collect()))
///     .build()?;
/// let page = catalog.search(&request).await?;
/// for artwork in page.value.data {
///     println!("{} ({})", artwork.title, artwork.id);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Catalog {
    inner: Arc<Inner>,
    /// Per handle, so one clone can wait longer than another (see [`Catalog::with_max_wait`]).
    max_wait: Duration,
}

#[derive(Debug)]
struct Inner {
    client: reqwest::Client,
    base_uri: String,
    cache: Cache,
    limiter: Arc<Limiter>,
    retries: u32,
}

impl Catalog {
    /// A builder that starts from the `ACRES_*` environment and the defaults.
    pub fn builder() -> CatalogBuilder {
        CatalogBuilder::default()
    }

    /// The API's base URI, such as `https://api.artic.edu/api/v1`.
    pub fn base_uri(&self) -> &str {
        &self.inner.base_uri
    }

    /// Whether this catalog only answers from the cache.
    pub fn is_offline(&self) -> bool {
        self.inner.cache.offline
    }

    /// The longest a request waits in the queue before it fails with
    /// [`ResponseError::Expired`].
    pub fn max_wait(&self) -> Duration {
        self.max_wait
    }

    /// Whether a request is waiting on the rate limit right now, for a status line.
    pub fn is_rate_limited(&self) -> bool {
        self.inner.limiter.is_starved()
    }

    /// A handle to the same catalog whose requests wait at most `max_wait`.
    pub fn with_max_wait(&self, max_wait: Duration) -> Catalog {
        Catalog {
            inner: Arc::clone(&self.inner),
            max_wait,
        }
    }

    /// Runs a search. Ask for [`ArtworkSummary::FIELDS`] so the response matches the type.
    pub async fn search(
        &self,
        request: &search::Request,
    ) -> Result<Fetched<Page<ArtworkSummary>>, Error> {
        self.get(request, Resource::Page).await
    }

    /// Lists artworks. Ask for [`ArtworkSummary::FIELDS`] so the response matches the type.
    pub async fn list(
        &self,
        request: &artworks::Request,
    ) -> Result<Fetched<Page<ArtworkSummary>>, Error> {
        self.get(request, Resource::Page).await
    }

    /// Fetches one artwork's full record.
    pub async fn artwork(&self, id: u32) -> Result<Fetched<Item<Artwork>>, Error> {
        let endpoint = format!(
            "{}/artworks/{id}?fields={}",
            self.inner.base_uri,
            Artwork::FIELDS.join(",")
        );
        self.get(endpoint, Resource::Record).await
    }

    /// Lists the museum's departments, to filter by.
    ///
    /// They change rarely, so they are cached as long as a [`Resource::Record`]. There are fewer
    /// than [`FACETS_LIMIT`], so one page holds them all.
    pub async fn departments(&self) -> Result<Fetched<Page<Department>>, Error> {
        self.get(
            self.facets("departments", Department::FIELDS),
            Resource::Record,
        )
        .await
    }

    /// Lists the artwork types, to filter by. Cached like [`Catalog::departments`].
    pub async fn artwork_types(&self) -> Result<Fetched<Page<ArtworkType>>, Error> {
        self.get(
            self.facets("artwork-types", ArtworkType::FIELDS),
            Resource::Record,
        )
        .await
    }

    fn facets(&self, resource: &str, fields: &[&str]) -> String {
        format!(
            "{}/{resource}?limit={FACETS_LIMIT}&fields={}",
            self.inner.base_uri,
            fields.join(",")
        )
    }

    /// Fetches an image.
    pub async fn image(&self, request: &iiif::ImageRequest) -> Result<Fetched<Bytes>, Error> {
        self.raw(request, Resource::Image).await
    }

    /// The image for `request` if it is in the cache, read without touching the network or the
    /// rate limit, even when online. With the cache turned off, nothing is cached.
    pub fn cached_image(&self, request: &iiif::ImageRequest) -> Result<Option<Bytes>, Error> {
        Ok(self
            .inner
            .cache
            .load(&request.to_string(), Resource::Image)?)
    }

    /// Whether the image for `request` is in the cache: one `stat`, with no read and no request.
    /// A cache file that cannot be checked counts as not cached.
    pub fn has_cached_image(&self, request: &iiif::ImageRequest) -> bool {
        self.inner
            .cache
            .contains(&request.to_string(), Resource::Image)
            .unwrap_or(false)
    }

    /// Fetches `endpoint` and parses the body as `T`.
    pub async fn get<T: DeserializeOwned>(
        &self,
        endpoint: impl Display,
        resource: Resource,
    ) -> Result<Fetched<T>, Error> {
        let endpoint = endpoint.to_string();
        let fetched = self.raw(&endpoint, resource).await?;
        let value =
            serde_json::from_slice(&fetched.value).map_err(|e| ParseError::body(endpoint, e))?;
        Ok(fetched.map(|_| value))
    }

    /// Fetches `endpoint` and returns the body as sent, for callers that pass it on unchanged.
    pub async fn raw(
        &self,
        endpoint: impl Display,
        resource: Resource,
    ) -> Result<Fetched<Bytes>, Error> {
        let endpoint = endpoint.to_string();
        let inner = &self.inner;
        if let Some(value) = inner.cache.load(&endpoint, resource)? {
            return Ok(Fetched {
                value,
                from_cache: true,
                wait: Duration::ZERO,
                fetch: Duration::ZERO,
            });
        }
        if inner.cache.offline {
            return Err(crate::CacheError::Missing { endpoint }.into());
        }

        let expired = || ResponseError::Expired {
            endpoint: endpoint.clone(),
        };
        let transport = |e: reqwest::Error| ResponseError::Transport {
            endpoint: endpoint.clone(),
            source: e.into(),
        };
        let deadline = Instant::now() + self.max_wait;
        let queued = Instant::now();
        let mut fetch = Duration::ZERO;
        let mut attempt = 0;
        loop {
            let permit = tokio::time::timeout_at(deadline, inner.limiter.acquire(resource.lane()))
                .await
                .map_err(|_| expired())?;
            let sent = Instant::now();
            let response = inner
                .client
                .get(&endpoint)
                .send()
                .await
                .map_err(transport)?;
            let status = response.status();
            let retry_after = retry_after(response.headers());
            let body = response.bytes().await.map_err(transport)?;
            fetch += sent.elapsed();
            drop(permit);

            if status.is_success() {
                if let Err(e) = inner.cache.store(&endpoint, resource, &body) {
                    tracing::warn!(error = %e, "could not cache response");
                }
                return Ok(Fetched {
                    value: body,
                    from_cache: false,
                    wait: queued.elapsed().saturating_sub(fetch),
                    fetch,
                });
            }

            let retryable = matches!(
                status,
                StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE
            );
            if retryable && attempt < inner.retries {
                let back_off = retry_after.unwrap_or(Duration::from_secs(1 << attempt));
                tracing::info!(%status, ?back_off, attempt, endpoint, "retrying");
                tokio::time::timeout_at(deadline, tokio::time::sleep(back_off))
                    .await
                    .map_err(|_| expired())?;
                attempt += 1;
                continue;
            }
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(ResponseError::TooManyRequests { endpoint }.into());
            }
            return Err(status_error(&endpoint, status, &body).into());
        }
    }
}

/// The `Retry-After` header as a delay, when it is given in seconds.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let seconds = headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(seconds))
}

/// An error status, with the API's `error` and `detail` when the body has them.
pub(crate) fn status_error(endpoint: &str, status: StatusCode, body: &[u8]) -> ResponseError {
    let json: Option<serde_json::Value> = serde_json::from_slice(body).ok();
    let field = |name: &str| {
        json.as_ref()
            .and_then(|json| json.get(name))
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };
    ResponseError::Status {
        endpoint: endpoint.to_string(),
        status: status.as_u16(),
        error: field("error")
            .unwrap_or_else(|| status.canonical_reason().unwrap_or("error").to_string()),
        detail: field("detail").unwrap_or_default(),
    }
}

/// An HTTP client that sends `AIC-User-Agent`, as the API asks, with `contact` when given.
pub(crate) fn http_client(contact: Option<&str>) -> Result<reqwest::Client, Error> {
    let agent = concat!("acres/", env!("CARGO_PKG_VERSION"));
    let aic_agent = match contact {
        Some(contact) => format!("{agent} ({contact})"),
        None => agent.to_string(),
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        "AIC-User-Agent",
        HeaderValue::from_str(&aic_agent)
            .map_err(|_| RequestError::InvalidContact(contact.unwrap_or_default().to_string()))?,
    );
    reqwest::Client::builder()
        .user_agent(agent)
        .default_headers(headers)
        .build()
        .map_err(|e| {
            ResponseError::Transport {
                endpoint: String::new(),
                source: e.into(),
            }
            .into()
        })
}

/// Builds a [`Catalog`]. Starts from the `ACRES_*` environment (`ACRES_BASE_URI`,
/// `ACRES_USE_CACHE`, `ACRES_CACHE_DIR`, `ACRES_OFFLINE`, `ACRES_CONTACT`) and the defaults.
#[derive(Clone, Debug)]
pub struct CatalogBuilder {
    base_uri: String,
    cache_dir: Option<PathBuf>,
    rate_limit_file: Option<PathBuf>,
    offline: bool,
    contact: Option<String>,
    max_wait: Duration,
    per_minute: u32,
    burst: u32,
    retries: u32,
    /// Why the `ACRES_*` environment could not be read, reported by [`CatalogBuilder::build`].
    config_error: Option<String>,
}

impl Default for CatalogBuilder {
    fn default() -> Self {
        let (config, config_error) = match Config::new() {
            Ok(config) => (Some(config), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let (base_uri, cache_dir, state_dir, offline, contact) = match config {
            Some(config) => (
                config.base_uri,
                config.use_cache.then_some(config.cache_dir),
                config.state_dir,
                config.offline,
                config.contact,
            ),
            None => (
                DEFAULT_BASE_URI.to_string(),
                Some(get_acres_cache_dir()),
                get_acres_state_dir(),
                false,
                None,
            ),
        };
        Self {
            base_uri,
            cache_dir,
            rate_limit_file: Some(state_dir.join(RATE_LIMIT_FILE)),
            offline,
            contact,
            max_wait: Duration::from_secs(10),
            per_minute: 60,
            burst: 5,
            retries: 3,
            config_error,
        }
    }
}

impl CatalogBuilder {
    /// Sets the API's base URI.
    pub fn base_uri(mut self, base_uri: impl Into<String>) -> Self {
        self.base_uri = base_uri.into();
        self
    }

    /// Caches responses in `dir`.
    pub fn cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(dir.into());
        self
    }

    /// Turns caching off: every request goes to the network.
    pub fn no_cache(mut self) -> Self {
        self.cache_dir = None;
        self
    }

    /// Shares the rate limit with every process that uses `path`. The default is `rate-limit`
    /// in the app's state directory (`ACRES_STATE_DIR`), so separate CLI runs and the TUI keep
    /// to one limit together, whether or not they cache.
    pub fn rate_limit_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.rate_limit_file = Some(path.into());
        self
    }

    /// Keeps the rate limit in memory, for this process only.
    pub fn no_shared_rate_limit(mut self) -> Self {
        self.rate_limit_file = None;
        self
    }

    /// Answers only from the cache, ignoring lifetimes. A miss is [`crate::CacheError::Missing`].
    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    /// Adds who to contact to the `AIC-User-Agent` header, such as an email address.
    pub fn contact(mut self, contact: Option<String>) -> Self {
        self.contact = contact;
        self
    }

    /// Sets how long a request may wait in the queue. Defaults to 10 seconds.
    pub fn max_wait(mut self, max_wait: Duration) -> Self {
        self.max_wait = max_wait;
        self
    }

    /// Sets the rate limit: `per_minute` requests a minute, at most `burst` at once.
    /// Defaults to 60 a minute, the API's anonymous limit, with bursts of 5.
    pub fn rate_limit(mut self, per_minute: u32, burst: u32) -> Self {
        self.per_minute = per_minute;
        self.burst = burst;
        self
    }

    /// Sets how many times a 429 or 503 is retried. Defaults to 3.
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Builds the catalog.
    ///
    /// Fails with [`ConfigError`] when an `ACRES_*` variable can't be read, such as
    /// `ACRES_OFFLINE=maybe`, rather than silently using the defaults.
    pub fn build(self) -> Result<Catalog, Error> {
        if let Some(message) = self.config_error {
            return Err(ConfigError::Invalid(message).into());
        }
        let mut limiter = Limiter::new(self.per_minute, self.burst);
        if let Some(path) = self.rate_limit_file {
            limiter = limiter.shared(SharedBucket::new(path));
        }
        Ok(Catalog {
            inner: Arc::new(Inner {
                client: http_client(self.contact.as_deref())?,
                base_uri: self.base_uri,
                cache: Cache::new(self.cache_dir, self.offline),
                limiter: Arc::new(limiter),
                retries: self.retries,
            }),
            max_wait: self.max_wait,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bad_environment_fails_the_build() {
        let builder = CatalogBuilder {
            config_error: Some("invalid type for key `offline`".into()),
            ..CatalogBuilder::default()
        };
        let error = builder.build().unwrap_err();
        assert_eq!(error.kind(), "config.invalid");
        assert!(error.to_string().contains("offline"), "{error}");
    }

    #[test]
    fn retry_after_reads_seconds() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("3"));
        assert_eq!(retry_after(&headers), Some(Duration::from_secs(3)));
    }

    #[test]
    fn retry_after_ignores_dates() {
        let mut headers = HeaderMap::new();
        headers.insert(
            RETRY_AFTER,
            HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
        );
        assert_eq!(retry_after(&headers), None);
    }

    #[test]
    fn status_error_reads_the_api_fields() {
        let error = status_error(
            "u",
            StatusCode::NOT_FOUND,
            br#"{"status": 404, "error": "Not found", "detail": "No such artwork"}"#,
        );
        assert!(matches!(
            error,
            ResponseError::Status { status: 404, ref error, ref detail, .. }
                if error == "Not found" && detail == "No such artwork"
        ));
    }

    #[test]
    fn status_error_falls_back_to_the_reason() {
        let error = status_error("u", StatusCode::BAD_GATEWAY, b"<html>");
        assert!(matches!(
            error,
            ResponseError::Status { status: 502, ref error, .. } if error == "Bad Gateway"
        ));
    }

    #[test]
    fn a_contact_with_a_newline_is_refused() {
        assert!(matches!(
            http_client(Some("a\nb")),
            Err(Error::Request(RequestError::InvalidContact(_)))
        ));
    }
}
