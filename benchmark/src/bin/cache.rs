use std::hint::black_box;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

use dashu::float::round::mode::HalfEven;
use dashu::float::FBig;

fn main() {
    let num_threads = 8;

    // We sweep from very small precisions (where locks dominate) to large ones (where math dominates)
    let precisions = [
        10, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000,
    ];

    println!("Finding the Mutex Lock Contention Turning Point (8 threads vs 1 thread)\n");
    println!(
        "{:<12} | {:<15} | {:<15} | {:<15}",
        "Precision", "1-Thread Time", "8-Thread Time", "Speedup"
    );
    println!("--------------------------------------------------------------------");

    for &precision in &precisions {
        // Clear the cache completely so each precision starts from a blank slate
        dashu::float::math::consts::clear_math_caches();

        // Dynamically scale iterations so small precisions do millions of loops (stressing the lock)
        // while large precisions do fewer loops (so we don't wait an hour).
        let iterations_per_thread = (5_000_000 / precision).max(10);
        let total_iterations = iterations_per_thread * num_threads;

        // --- 1-Thread Baseline ---
        // Doing the TOTAL workload sequentially
        let start_single = Instant::now();
        for _ in 0..total_iterations {
            black_box(FBig::<HalfEven, 2>::pi(black_box(precision)));
        }
        let duration_single = start_single.elapsed();

        // --- 8-Thread Contention Test ---
        // Clean cache again so the 8 threads also suffer the Cache Miss simultaneously
        dashu::float::math::consts::clear_math_caches();

        // Doing the SAME total workload, but split across 8 threads pounding the Mutex simultaneously
        let barrier = Arc::new(Barrier::new(num_threads + 1));
        let mut handles = vec![];

        for _ in 0..num_threads {
            let b = Arc::clone(&barrier);
            let iters = iterations_per_thread;
            let handle = thread::spawn(move || {
                b.wait(); // Release the hounds!
                for _ in 0..iters {
                    black_box(FBig::<HalfEven, 2>::pi(black_box(precision)));
                }
            });
            handles.push(handle);
        }

        barrier.wait();
        let start_multi = Instant::now();
        for handle in handles {
            handle.join().unwrap();
        }
        let duration_multi = start_multi.elapsed();

        let ms_single = duration_single.as_secs_f64() * 1000.0;
        let ms_multi = duration_multi.as_secs_f64() * 1000.0;
        let speedup = ms_single / ms_multi;

        let ms_single_str = format!("{:.1}ms", ms_single);
        let ms_multi_str = format!("{:.1}ms", ms_multi);
        let speedup_str = format!("{:.2}x", speedup);

        println!(
            "{:<12} | {:<15} | {:<15} | {:<15}",
            precision, ms_single_str, ms_multi_str, speedup_str
        );
    }
}
