use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use plan9whack::unwhack::unwhack;
use plan9whack::whack::whackblock;

fn bench_whack_unwhack(c: &mut Criterion) {
    let mut group = c.benchmark_group("whack");
    // trivially compressible input: 64KiB zeros
    let compressible = vec![0u8; 65_536];
    group.bench_with_input(
        BenchmarkId::new("whackblock", "zeros-64k"),
        &compressible,
        |b, data| {
            b.iter(|| {
                let compressed = whackblock(data).expect("should compress");
                // decompress to validate and include unwhack in the bench iteration
                let decompressed = unwhack(&compressed, data.len()).expect("decompress");
                assert_eq!(decompressed.len(), data.len());
            })
        },
    );

    // 64 KiB limited alphabet
    let mut data = Vec::with_capacity(65_536);
    let vocab = ["green", "eggs", "and", "ham", "sam", "i", "am"];

    for i in 0..65_536 {
        let index = ((i * 2654435761u32) >> 16) % vocab.len() as u32;
        data.extend_from_slice(vocab[index as usize].as_bytes());
    }
    group.bench_with_input(
        BenchmarkId::new("whackblock", "limited-vocab"),
        &data,
        |b, data| {
            b.iter(|| {
                let compressed = whackblock(data).expect("should compress");
                // decompress to validate and include unwhack in the bench iteration
                let decompressed = unwhack(&compressed, data.len()).expect("decompress");
                assert_eq!(decompressed.len(), data.len());
            })
        },
    );

    // pseudo-random / less compressible input (deterministic)
    let mut pattern = Vec::with_capacity(65_536);
    for i in 0..65_536 {
        pattern.push(((i * 2654435761u32) >> 16) as u8);
    }
    group.bench_with_input(
        BenchmarkId::new("whackblock", "pseudo-random-64k"),
        &pattern,
        |b, data| {
            b.iter(|| {
                // whackblock may return None for incompressible data; just run it to measure attempt cost
                let _ = whackblock(data);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench_whack_unwhack);
criterion_main!(benches);
