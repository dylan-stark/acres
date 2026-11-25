//! `Catalog` against a mock API (ADR 0016). No test reaches the real API.

use std::time::Duration;

use acres::{
    CacheError, Catalog, Error, Resource, ResponseError,
    artworks::{
        ArtworkSummary,
        request::search::{self, Filters},
    },
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

const SEARCH_PAGE: &str = include_str!("fixtures/search_page.json");

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("acres-catalog-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

async fn server() -> (MockServer, String) {
    let server = MockServer::start().await;
    let base = format!("{}/api/v1", server.uri());
    (server, base)
}

fn catalog(base: &str) -> Catalog {
    Catalog::builder()
        .no_shared_rate_limit()
        .base_uri(base)
        .no_cache()
        .offline(false)
        .contact(None)
        .build()
        .unwrap()
}

fn search_request(base: &str) -> search::Request {
    search::Request::builder()
        .base_uri(base)
        .q(Some("monet".into()))
        .fields(Some(
            ArtworkSummary::FIELDS
                .iter()
                .map(|f| f.to_string())
                .collect(),
        ))
        .limit(Some(50))
        .build()
        .unwrap()
}

#[tokio::test]
async fn search_parses_a_page_of_summaries() {
    let (server, base) = server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/artworks/search"))
        .and(query_param("q", "monet"))
        .and(query_param("limit", "50"))
        .and(query_param("fields", ArtworkSummary::FIELDS.join(",")))
        .respond_with(ResponseTemplate::new(200).set_body_string(SEARCH_PAGE))
        .expect(1)
        .mount(&server)
        .await;

    let page = catalog(&base).search(&search_request(&base)).await.unwrap();

    assert!(!page.from_cache);
    assert_eq!(page.value.pagination.total, 1204);
    assert_eq!(page.value.data.len(), 50);
    let first = &page.value.data[0];
    assert_eq!(first.title, "Study No. 1");
    assert!(
        first
            .artist_display
            .as_deref()
            .unwrap()
            .starts_with("Claude Monet")
    );
    assert!(first.thumbnail.as_ref().unwrap().lqip.is_some());
    assert_eq!(page.value.config.iiif_url, "https://www.artic.edu/iiif/2");
}

#[tokio::test]
async fn filters_reach_the_api_in_bracket_form() {
    let (server, base) = server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/artworks/search"))
        .and(query_param(
            "query[bool][filter][0][exists][field]",
            "image_id",
        ))
        .and(query_param(
            "query[bool][filter][1][term][is_public_domain]",
            "true",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SEARCH_PAGE))
        .expect(1)
        .mount(&server)
        .await;

    let request = search::Request::builder()
        .base_uri(base.clone())
        .filters(Filters {
            has_image: true,
            is_public_domain: Some(true),
            ..Filters::default()
        })
        .build()
        .unwrap();
    catalog(&base).search(&request).await.unwrap();
}

#[tokio::test]
async fn artwork_asks_for_its_fields() {
    let (server, base) = server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/artworks/4"))
        .and(query_param("fields", acres::artworks::Artwork::FIELDS.join(",")))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"data": {"id": 4, "title": "Priest and Boy", "medium_display": "Oil on canvas",
                "credit_line": "Gift", "is_on_view": false},
                "config": {"iiif_url": "https://www.artic.edu/iiif/2", "website_url": "https://www.artic.edu"}}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let artwork = catalog(&base).artwork(4).await.unwrap().value.data;
    assert_eq!(artwork.summary.title, "Priest and Boy");
    assert_eq!(artwork.medium_display.as_deref(), Some("Oil on canvas"));
}

#[tokio::test]
async fn sends_aic_user_agent_without_a_contact() {
    let (server, base) = server().await;
    let agent = concat!("acres/", env!("CARGO_PKG_VERSION"));
    Mock::given(header("AIC-User-Agent", agent))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;

    catalog(&base)
        .raw(format!("{base}/artworks/4"), Resource::Record)
        .await
        .unwrap();
}

#[tokio::test]
async fn sends_aic_user_agent_with_a_contact() {
    let (server, base) = server().await;
    let agent = concat!("acres/", env!("CARGO_PKG_VERSION"), " (me@example.com)");
    Mock::given(header("AIC-User-Agent", agent))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;

    Catalog::builder()
        .no_shared_rate_limit()
        .base_uri(&base)
        .no_cache()
        .contact(Some("me@example.com".into()))
        .build()
        .unwrap()
        .raw(format!("{base}/artworks/4"), Resource::Record)
        .await
        .unwrap();
}

#[tokio::test]
async fn retries_a_429_after_retry_after() {
    let (server, base) = server().await;
    Mock::given(path("/api/v1/artworks/4"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .up_to_n_times(2)
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/artworks/4"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;

    let fetched = catalog(&base)
        .raw(format!("{base}/artworks/4"), Resource::Record)
        .await
        .unwrap();
    assert_eq!(fetched.value, "{}");
}

#[tokio::test]
async fn gives_up_after_the_retries() {
    let (server, base) = server().await;
    Mock::given(path("/api/v1/artworks/4"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .expect(3)
        .mount(&server)
        .await;

    let error = Catalog::builder()
        .no_shared_rate_limit()
        .base_uri(&base)
        .no_cache()
        .retries(2)
        .build()
        .unwrap()
        .raw(format!("{base}/artworks/4"), Resource::Record)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        Error::Response(ResponseError::TooManyRequests { .. })
    ));
}

#[tokio::test]
async fn an_error_status_carries_the_api_fields() {
    let (server, base) = server().await;
    Mock::given(path("/api/v1/artworks/0"))
        .respond_with(ResponseTemplate::new(404).set_body_string(
            r#"{"status": 404, "error": "Not found", "detail": "The item you requested cannot be found."}"#,
        ))
        .mount(&server)
        .await;

    let error = catalog(&base).artwork(0).await.unwrap_err();
    assert!(matches!(
        error,
        Error::Response(ResponseError::Status { status: 404, ref error, .. }) if error == "Not found"
    ));
}

#[tokio::test]
async fn a_request_past_its_max_wait_expires_unsent() {
    let (server, base) = server().await;
    Mock::given(path("/api/v1/artworks/4"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;

    // One token, earned back once a minute: the second request can't be sent within 50 ms.
    let catalog = Catalog::builder()
        .no_shared_rate_limit()
        .base_uri(&base)
        .no_cache()
        .rate_limit(1, 1)
        .max_wait(Duration::from_millis(50))
        .build()
        .unwrap();
    let url = format!("{base}/artworks/4");
    catalog.raw(&url, Resource::Record).await.unwrap();
    let error = catalog.raw(&url, Resource::Record).await.unwrap_err();
    assert!(matches!(
        error,
        Error::Response(ResponseError::Expired { .. })
    ));
}

#[tokio::test]
async fn a_cached_response_is_not_fetched_again() {
    let (server, base) = server().await;
    Mock::given(path("/api/v1/artworks/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SEARCH_PAGE))
        .expect(1)
        .mount(&server)
        .await;

    let catalog = Catalog::builder()
        .no_shared_rate_limit()
        .base_uri(&base)
        .cache_dir(temp_dir("cached"))
        .build()
        .unwrap();
    let request = search_request(&base);
    assert!(!catalog.search(&request).await.unwrap().from_cache);
    assert!(catalog.search(&request).await.unwrap().from_cache);
}

#[tokio::test]
async fn offline_answers_from_the_cache_only() {
    let (server, base) = server().await;
    Mock::given(path("/api/v1/artworks/4"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;
    let dir = temp_dir("offline");
    let online = Catalog::builder()
        .no_shared_rate_limit()
        .base_uri(&base)
        .cache_dir(&dir)
        .build()
        .unwrap();
    online
        .raw(format!("{base}/artworks/4"), Resource::Record)
        .await
        .unwrap();

    let offline = Catalog::builder()
        .no_shared_rate_limit()
        .base_uri(&base)
        .cache_dir(&dir)
        .offline(true)
        .build()
        .unwrap();
    let hit = offline
        .raw(format!("{base}/artworks/4"), Resource::Record)
        .await
        .unwrap();
    assert!(hit.from_cache);
    let miss = offline
        .raw(format!("{base}/artworks/5"), Resource::Record)
        .await
        .unwrap_err();
    assert!(matches!(miss, Error::Cache(CacheError::Missing { .. })));
}

#[tokio::test]
async fn raw_returns_the_body_unchanged() {
    let (server, base) = server().await;
    let body = r#"{"data": {"id": 4, "extra": [1, 2, 3]}}"#;
    Mock::given(path("/api/v1/artworks/4"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let fetched = catalog(&base)
        .raw(format!("{base}/artworks/4"), Resource::Record)
        .await
        .unwrap();
    assert_eq!(fetched.value, body);
}

#[tokio::test]
async fn catalogs_on_one_rate_limit_file_share_the_rate_limit() {
    let (server, base) = server().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;

    // Like two CLI runs: separate catalogs, one state file, one token a minute between them.
    let file = temp_dir("shared-limit").join("rate-limit");
    let catalog = || {
        Catalog::builder()
            .base_uri(&base)
            .no_cache()
            .rate_limit_file(&file)
            .rate_limit(1, 1)
            .max_wait(Duration::from_millis(50))
            .build()
            .unwrap()
    };
    catalog()
        .raw(format!("{base}/artworks/1"), Resource::Record)
        .await
        .unwrap();
    let error = catalog()
        .raw(format!("{base}/artworks/2"), Resource::Record)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        Error::Response(ResponseError::Expired { .. })
    ));
}
