//! Whole frames of the explorer screen (ADR 0013): `terminal.draw` into a `TestBackend`.
//!
//! Each iteration alternates the terminal between 200x60 and 199x60, so the art re-renders every
//! frame, the worst case and the one the 8 ms budget is about. The list holds the five fixture
//! artworks, and the art is La Grande Jatte (843x565).

use std::time::Duration;

use acres_tui::{
    action::Action, components::Component, components::home::Home, config::Config,
    explorer::Explorer,
};
use charaoke::ColorDepth;
use criterion::{Criterion, criterion_group, criterion_main};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

const ARTWORKS: &str = include_str!("../tests/fixtures/artworks.json");
const IMAGE: &[u8] = include_bytes!(
    "../tests/fixtures/iiif/2d484387-2509-5e8e-2c43-22f9981972eb/full/843,/0/default.jpg"
);

/// An explorer showing the fixture image, with art mode cycled `cycles` times from half blocks.
fn explorer(cycles: usize, keys: &[Action]) -> Explorer {
    let config = Config::new().expect("default config");
    let artworks = serde_json::from_str(ARTWORKS).expect("fixture parses");
    let mut explorer =
        Explorer::with_art(artworks, &config, Home::with_depth(ColorDepth::TrueColor));
    explorer
        .update(Action::UpdateImage {
            id: 27992,
            image: IMAGE.into(),
            fetch: Duration::ZERO,
        })
        .unwrap();
    for _ in 0..cycles {
        explorer.update(Action::CycleArtMode).unwrap();
    }
    for action in keys {
        explorer.update(action.clone()).unwrap();
    }
    explorer
}

fn bench_frames(c: &mut Criterion, name: &str, keys: &[Action], modes: &[(&str, usize)]) {
    let mut group = c.benchmark_group(format!("tui/frame/{name}"));
    for &(mode, cycles) in modes {
        let mut explorer = explorer(cycles, keys);
        let mut terminal = Terminal::new(TestBackend::new(200, 60)).unwrap();
        let mut wide = true;
        group.bench_function(mode, |b| {
            b.iter(|| {
                wide = !wide;
                let width = if wide { 200 } else { 199 };
                terminal.resize(Rect::new(0, 0, width, 60)).unwrap();
                terminal
                    .draw(|frame| explorer.draw(frame, frame.area()).unwrap())
                    .unwrap();
            })
        });
    }
    group.finish();
}

/// Half blocks are the default, then each cycle steps to quadrant blocks and ASCII.
const MODES: &[(&str, usize)] = &[("half", 0), ("quadrant", 1), ("ascii", 2)];

fn frames(c: &mut Criterion) {
    bench_frames(c, "explorer", &[], MODES);
    bench_frames(c, "zen", &[Action::ToggleZen], MODES);
    bench_frames(c, "help", &[Action::ToggleHelp], &[("ascii", 2)]);
}

criterion_group!(benches, frames);
criterion_main!(benches);
