use std::fs;
use std::str::FromStr;

use criterion::{criterion_group, criterion_main, Criterion};

use fgddem::dataset;

fn bench(c: &mut Criterion) {
    let content =
        std::fs::read_to_string("./tests/fixture/FG-GML-5238-74-00-DEM5A-20161001.xml").unwrap();

    c.bench_function("parse xml", |b| {
        b.iter(|| {
            let dataset = dataset::Dataset::from_str(&content).unwrap();
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
