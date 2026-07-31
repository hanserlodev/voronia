use criterion::{black_box, criterion_group, criterion_main, Criterion};

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vor-import/tests/reference/Sorvik-2026-07-24-23-39.map"
);

fn bench_import_map(c: &mut Criterion) {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("Sorvik map must exist");
    let raw = vor_import::mapfile::raw::parse(&bytes).expect("raw parse");

    let mut group = c.benchmark_group("import_map");
    group.sample_size(10);
    group.bench_function("full_import", |b| {
        b.iter(|| {
            let result = black_box(vor_import::mapfile::Loader::load(&raw)).expect("load");
            black_box(result.world);
        })
    });
    group.finish();
}

fn bench_vorn_roundtrip(c: &mut Criterion) {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("Sorvik map must exist");
    let raw = vor_import::mapfile::raw::parse(&bytes).expect("raw parse");
    let result = vor_import::mapfile::Loader::load(&raw).expect("load");
    let world = &result.world;

    let tmp = std::env::temp_dir().join("bench_save.vorn");
    let metadata = vor_format::VornMetadata::new(
        &world.settings.map_name,
        &world.header.seed,
        &world.header.date,
        env!("CARGO_PKG_VERSION"),
        Some(&world.header.version),
    );

    let mut group = c.benchmark_group("vorn_save_load");
    group.sample_size(10);

    group.bench_function("save_vorn", |b| {
        b.iter(|| {
            vor_format::save::save(black_box(&tmp), black_box(world), black_box(&metadata))
                .expect("save");
        })
    });

    group.bench_function("load_vorn", |b| {
        b.iter(|| {
            let (w, m) = vor_format::load::load(black_box(&tmp)).expect("load");
            black_box(w);
            black_box(m);
        })
    });

    group.finish();
    std::fs::remove_file(&tmp).ok();
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(std::time::Duration::from_secs(10));
    targets = bench_import_map, bench_vorn_roundtrip
}
criterion_main!(benches);
