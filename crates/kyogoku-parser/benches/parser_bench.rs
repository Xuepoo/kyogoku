use criterion::{Criterion, black_box, criterion_group, criterion_main};
use kyogoku_parser::ParserRegistry;
use std::path::Path;

fn bench_parsers(c: &mut Criterion) {
    let registry = ParserRegistry::new();

    // We include bytes relative to the source file
    // benches/parser_bench.rs -> ../tests/fixtures/...
    let rpy_bytes = include_bytes!("../tests/fixtures/games/basic_dialogue.rpy");
    let ass_bytes = include_bytes!("../tests/fixtures/subtitles/effect_tags.ass");
    let srt_bytes = include_bytes!("../tests/fixtures/subtitles/standard.srt");
    let json_bytes = include_bytes!("../tests/fixtures/games/mtool_export.json");

    let mut group = c.benchmark_group("parser_benchmarks");

    group.bench_function("parse_rpy", |b| {
        b.iter(|| {
            let parser = registry.get_parser(Path::new("test.rpy")).unwrap();
            parser.parse(black_box(rpy_bytes)).unwrap();
        })
    });

    group.bench_function("parse_ass", |b| {
        b.iter(|| {
            let parser = registry.get_parser(Path::new("test.ass")).unwrap();
            parser.parse(black_box(ass_bytes)).unwrap();
        })
    });

    group.bench_function("parse_srt", |b| {
        b.iter(|| {
            let parser = registry.get_parser(Path::new("test.srt")).unwrap();
            parser.parse(black_box(srt_bytes)).unwrap();
        })
    });

    group.bench_function("parse_json", |b| {
        b.iter(|| {
            let parser = registry.get_parser(Path::new("test.json")).unwrap();
            parser.parse(black_box(json_bytes)).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_parsers);
criterion_main!(benches);
