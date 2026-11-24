//! Parsing a page of search results into typed records (ADR 0016).
//!
//! Far from any frame budget, but it keeps the model cheap as fields are added.

use acres::{Page, artworks::ArtworkSummary};
use criterion::{Criterion, criterion_group, criterion_main};

const SEARCH_PAGE: &str = include_str!("../tests/fixtures/search_page.json");

fn parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("acres/parse");
    group.bench_function("search_page", |b| {
        b.iter(|| {
            let page: Page<ArtworkSummary> =
                serde_json::from_str(std::hint::black_box(SEARCH_PAGE)).unwrap();
            page
        })
    });
    group.finish();
}

criterion_group!(benches, parse);
criterion_main!(benches);
