//! Browsing over the disk cache (ADR 0018).
//!
//! `check_page` asks which of a page of 50 artworks have their image cached, half of them cached:
//! one `stat` each, on a blocking thread after every list load. `cached` reads one cached
//! 843-pixel image (La Grande Jatte, 843x565) and decodes it, the work done off the loop once the
//! cursor rests on a cached artwork, debounce excluded.

use std::path::PathBuf;

use acres::{ApiConfig, Catalog, artworks::ArtworkSummary};
use acres_tui::{
    components::{home::Decoded, iiif::cached_ids},
    entry::Entry,
};
use criterion::{Criterion, criterion_group, criterion_main};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

const IMAGE: &[u8] = include_bytes!(
    "../tests/fixtures/iiif/2d484387-2509-5e8e-2c43-22f9981972eb/full/843,/0/default.jpg"
);

/// Rows on a page, as the explorer loads them.
const PAGE: u32 = 50;

fn entry(server: &str, id: u32) -> Entry {
    Entry {
        summary: ArtworkSummary {
            id,
            image_id: Some(format!("image-{id}")),
            ..ArtworkSummary::default()
        },
        config: ApiConfig {
            iiif_url: format!("{server}/iiif"),
            ..ApiConfig::default()
        },
    }
}

/// A page of entries and a catalog whose cache holds the even ones' images, fetched from a
/// local mock server once.
fn setup() -> (Vec<Entry>, Catalog) {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("acres-tui-bench-browse-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(IMAGE))
            .mount(&server)
            .await;
        let catalog = Catalog::builder()
            .base_uri(server.uri())
            .cache_dir(&dir)
            .no_shared_rate_limit()
            .rate_limit(60_000, PAGE)
            .offline(false)
            .build()
            .unwrap();
        let entries: Vec<Entry> = (0..PAGE).map(|id| entry(&server.uri(), id)).collect();
        for entry in entries.iter().step_by(2) {
            catalog
                .image(&entry.image_request().unwrap())
                .await
                .unwrap();
        }
        (entries, catalog)
    })
}

fn browse(c: &mut Criterion) {
    let (entries, catalog) = setup();
    assert_eq!(cached_ids(&catalog, &entries).len(), PAGE as usize / 2);
    let request = entries[0].image_request().unwrap();
    let mut group = c.benchmark_group("tui/browse");
    group.bench_function("check_page", |b| {
        b.iter(|| cached_ids(&catalog, std::hint::black_box(&entries)))
    });
    group.bench_function("cached", |b| {
        b.iter(|| {
            let bytes = catalog.cached_image(&request).unwrap().unwrap();
            Decoded::decode(&bytes).unwrap()
        })
    });
    group.finish();
}

criterion_group!(benches, browse);
criterion_main!(benches);
