//! What a `Catalog` request logs (ADR 0020), against a mock API. This is its own test binary
//! because `tracing` caches which callsites are enabled per process: tests in the same binary
//! that log with no subscriber would switch these events off.

use acres::{
    Catalog,
    artworks::{ArtworkSummary, request::search},
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

const SEARCH_PAGE: &str = include_str!("fixtures/search_page.json");

/// Collects the `message` and `kind` of every event, for checking what a request logs.
#[derive(Clone, Default)]
struct Events(std::sync::Arc<std::sync::Mutex<Vec<Logged>>>);

/// An event's `message` and its `kind` field, if it has one.
type Logged = (String, Option<String>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Events {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        #[derive(Default)]
        struct Fields(String, Option<String>);
        impl tracing::field::Visit for Fields {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if field.name() == "kind" {
                    self.1 = Some(value.to_string());
                }
            }
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{value:?}");
                }
            }
        }
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.0.lock().unwrap().push((fields.0, fields.1));
    }
}

#[tokio::test]
async fn each_request_ends_with_one_event() {
    use tracing_subscriber::layer::SubscriberExt;

    let server = MockServer::start().await;
    let base = format!("{}/api/v1", server.uri());
    Mock::given(path("/api/v1/artworks/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SEARCH_PAGE))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/artworks/404"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let events = Events::default();
    let subscriber = tracing_subscriber::registry().with(events.clone());
    let _guard = tracing::subscriber::set_default(subscriber);
    let catalog = Catalog::builder()
        .no_shared_rate_limit()
        .base_uri(&base)
        .no_cache()
        .offline(false)
        .build()
        .unwrap();
    let request = search::Request::builder()
        .base_uri(&base)
        .q(Some("monet".into()))
        .fields(Some(
            ArtworkSummary::FIELDS
                .iter()
                .map(|f| f.to_string())
                .collect(),
        ))
        .build()
        .unwrap();
    catalog.search(&request).await.unwrap();
    catalog.artwork(404).await.unwrap_err();

    let ends: Vec<_> = events
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|(message, _)| message == "fetched" || message == "request failed")
        .cloned()
        .collect();
    assert_eq!(
        ends,
        [
            ("fetched".to_string(), None),
            (
                "request failed".to_string(),
                Some("response.not_found".to_string())
            ),
        ]
    );
}
