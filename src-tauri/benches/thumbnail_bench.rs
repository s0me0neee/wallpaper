use criterion::{black_box, BenchmarkId, Criterion, Throughput};
use rayon::prelude::*;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use wallpaper_lib::thumbnail;

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "gif"];

// ── helpers ──────────────────────────────────────────────────────────────────

fn all_images() -> Vec<PathBuf> {
    let dir = dirs::home_dir().expect("no home dir").join("Pictures/wallpaper");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        eprintln!("warn: {} not found — no images to benchmark", dir.display());
        return vec![];
    };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .collect()
}

fn pct(sorted: &[u128], p: usize) -> f64 {
    sorted[(sorted.len() * p / 100).min(sorted.len() - 1)] as f64 / 1000.0
}

fn mean_stddev(sorted: &[u128]) -> (f64, f64) {
    let n = sorted.len() as f64;
    let mean = sorted.iter().sum::<u128>() as f64 / n;
    let var = sorted.iter().map(|&t| { let d = t as f64 - mean; d * d }).sum::<f64>() / n;
    (mean / 1000.0, var.sqrt() / 1000.0)
}

// ── stats report (phase 1) ───────────────────────────────────────────────────

fn stats_report(all: &[PathBuf], threads: usize, pool: &rayon::ThreadPool) {
    let w = 54usize;
    let title = format!("Thumbnail Benchmark  ({} images)", all.len());
    eprintln!("\n┌{:─<w$}┐", "");
    eprintln!("│  {title:<w$}│", w = w - 2);
    eprintln!("└{:─<w$}┘", "");

    // ── sequential ──
    eprintln!("\n  Sequential  (per-image)");
    eprintln!("  {:─<w$}", "");
    let mut seq_us: Vec<u128> = all
        .iter()
        .filter_map(|p| {
            let t = Instant::now();
            thumbnail::generate(p).map(|_| t.elapsed().as_micros())
        })
        .collect();
    seq_us.sort_unstable();

    if !seq_us.is_empty() {
        let n = seq_us.len();
        let total_ms = seq_us.iter().sum::<u128>() / 1000;
        let (mean, stddev) = mean_stddev(&seq_us);
        eprintln!(
            "  {:>4} images   total {:>7} ms   {:>5.1} img/s",
            n, total_ms, n as f64 / (total_ms as f64 / 1000.0)
        );
        eprintln!("  Mean {:>7.1} ms   Stddev {:>7.1} ms", mean, stddev);
        eprintln!(
            "  p25  {:>7.1} ms   p50    {:>7.1} ms   p75 {:>7.1} ms",
            pct(&seq_us, 25), pct(&seq_us, 50), pct(&seq_us, 75)
        );
        eprintln!(
            "  Min  {:>7.1} ms   Max    {:>7.1} ms",
            seq_us[0] as f64 / 1000.0,
            seq_us[n - 1] as f64 / 1000.0
        );
    }

    // ── parallel ──
    eprintln!("\n  Parallel  ({threads} threads, wall-clock)");
    eprintln!("  {:─<w$}", "");
    let wall = Instant::now();
    let mut par_us: Vec<u128> = pool.install(|| {
        all.par_iter()
            .filter_map(|p| {
                let t = Instant::now();
                thumbnail::generate(p).map(|_| t.elapsed().as_micros())
            })
            .collect()
    });
    let wall_ms = wall.elapsed().as_millis();
    par_us.sort_unstable();

    if !par_us.is_empty() {
        let n = par_us.len();
        let cpu_ms = par_us.iter().sum::<u128>() / 1000;
        let speedup = cpu_ms as f64 / wall_ms as f64;
        let (mean, stddev) = mean_stddev(&par_us);
        eprintln!(
            "  {:>4} images   total {:>7} ms   {:>5.1} img/s",
            n, wall_ms, n as f64 / (wall_ms as f64 / 1000.0)
        );
        eprintln!(
            "  CPU  {:>7} ms   speedup {:>4.1}x  ({:.0}% of linear)",
            cpu_ms, speedup, speedup / threads as f64 * 100.0
        );
        eprintln!("  Mean {:>7.1} ms   Stddev {:>7.1} ms  (per-thread)", mean, stddev);
        eprintln!(
            "  p25  {:>7.1} ms   p50    {:>7.1} ms   p75 {:>7.1} ms",
            pct(&par_us, 25), pct(&par_us, 50), pct(&par_us, 75)
        );
    }

    eprintln!();
}

// ── criterion groups (phase 2) ───────────────────────────────────────────────

fn bench_sequential(c: &mut Criterion, all: &[PathBuf]) {
    let mut group = c.benchmark_group("sequential");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(15));

    for n in [1usize, 4, 10] {
        let batch: Vec<_> = all.iter().take(n).collect();
        if batch.len() < n { break; }
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &batch, |b, batch| {
            b.iter(|| {
                for p in batch.iter() {
                    thumbnail::generate(black_box(p));
                }
            })
        });
    }
    group.finish();
}

fn bench_parallel(
    c: &mut Criterion,
    all: &[PathBuf],
    threads: usize,
    pool: &rayon::ThreadPool,
) {
    let mut group = c.benchmark_group("parallel");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(15));

    for n in [4usize, 10, all.len()] {
        let batch: Vec<_> = all.iter().take(n).collect();
        if batch.len() < n { break; }
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(
            BenchmarkId::new(format!("{threads}t"), n),
            &batch,
            |b, batch| {
                b.iter_custom(|iters| {
                    let mut total = Duration::ZERO;
                    for _ in 0..iters {
                        let start = Instant::now();
                        pool.install(|| {
                            batch.par_iter().for_each(|p| { thumbnail::generate(black_box(p)); });
                        });
                        total += start.elapsed();
                    }
                    total
                })
            },
        );
    }
    group.finish();
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    let all = all_images();
    if all.is_empty() {
        eprintln!("No images found — nothing to benchmark.");
        return;
    }

    let threads = std::thread::available_parallelism()
        .map(|n| n.get().clamp(2, 6))
        .unwrap_or(4);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("failed to build rayon pool");

    // Phase 1: one-shot stats with percentiles
    stats_report(&all, threads, &pool);

    // Phase 2: criterion for rigorous repeated measurements
    let sep = "─".repeat(54);
    eprintln!("  {sep}");
    eprintln!("  Criterion  (confidence intervals, HTML report)");
    eprintln!("  {sep}\n");

    let mut c = Criterion::default().configure_from_args();
    bench_sequential(&mut c, &all);
    bench_parallel(&mut c, &all, threads, &pool);
    c.final_summary();
}
