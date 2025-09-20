use std::{hint::black_box, time::Duration};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use libspy::{
    entry::Entry,
    template::{Template, context_value},
};
use once_cell::sync::Lazy;

static ENTRY_FIXTURE: &str = include_str!("fixtures/entry.json");
static COMPLEX_TEMPLATE_FIXTURE: &str = include_str!("fixtures/render_full.j2");

static ENTRY: Lazy<Entry> = Lazy::new(|| {
    serde_json::from_str::<Entry>(ENTRY_FIXTURE)
        .expect("fixture entry JSON should deserialize into Entry")
});

static SIMPLE_TEMPLATE: Lazy<Template> = Lazy::new(|| Template::new("{{ title }}".to_owned()));
static COMPLEX_TEMPLATE: Lazy<Template> =
    Lazy::new(|| Template::new(COMPLEX_TEMPLATE_FIXTURE.to_owned()));

struct Scenario {
    id: &'static str,
    template: &'static Template,
    bytes_out: usize,
}

static SCENARIOS: Lazy<[Scenario; 2]> = Lazy::new(|| {
    let entry = &*ENTRY;

    [
        Scenario {
            id: "title_only",
            template: &*SIMPLE_TEMPLATE,
            bytes_out: SIMPLE_TEMPLATE
                .render(entry)
                .expect("title template renders for sizing")
                .len(),
        },
        Scenario {
            id: "full_document",
            template: &*COMPLEX_TEMPLATE,
            bytes_out: COMPLEX_TEMPLATE
                .render(entry)
                .expect("full template renders for sizing")
                .len(),
        },
    ]
});

fn render_templates(c: &mut Criterion) {
    let entry = &*ENTRY;
    let mut group = c.benchmark_group("template_render");

    // Tighten confidence intervals for microsecond-scale work.
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(200)
        .noise_threshold(0.01);

    for scenario in SCENARIOS.iter() {
        group.throughput(Throughput::Bytes(scenario.bytes_out as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(scenario.id),
            scenario,
            |b, scenario| {
                b.iter_with_large_drop(|| {
                    scenario
                        .template
                        .render(black_box(entry))
                        .expect("template renders")
                });
            },
        );
    }

    group.finish();
}

fn build_context_value(c: &mut Criterion) {
    let entry = &*ENTRY;
    c.bench_function("template_context/value_from_entry", |b| {
        b.iter(|| black_box(context_value(entry)));
    });
}

fn configure_criterion() -> Criterion {
    Criterion::default()
        .configure_from_args()
        .confidence_level(0.99)
        .significance_level(0.01)
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets = render_templates, build_context_value
}
criterion_main!(benches);
