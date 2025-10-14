//! Radix sort implementation for investigating GPU sorting algorithms.
//! Should be replaced with a wgsl version in the future.

use std::{collections::HashMap, time::Duration};

pub fn radix_sort(input: &mut Vec<u32>) {
    if input.len() <= 1 {
        return;
    }

    let n = input.len();
    let mut out_buf = vec![0u32; n];
    let mut count = [0usize; 256];

    // 4 byte-wise passes: 0, 8, 16, 24
    for shift in (0..32).step_by(8) {
        // Counting phase
        count.fill(0);
        for &x in input.iter() {
            count[((x >> shift) & 0xFF) as usize] += 1;
        }

        // Exclusive prefix sums -> starting indices for each bucket
        let mut sum = 0usize;
        for c in count.iter_mut() {
            let tmp = *c;
            *c = sum;
            sum += tmp;
        }

        // Distribute (forward iteration is fine with exclusive sums)
        for (i, &x) in input.iter().enumerate() {
            let b = ((x >> shift) & 0xFF) as usize;
            let pos = count[b];
            out_buf[pos] = x;
            count[b] = pos + 1;
        }

        // Next pass: swap which buffer is read vs written
        std::mem::swap(input, &mut out_buf);
    }
}

fn build_vec(size: usize) -> Vec<u32> {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..size).map(|_| rng.random()).collect()
}

struct RunTime(Vec<Duration>);

impl RunTime {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn add(&mut self, duration: Duration) {
        self.0.push(duration);
    }

    fn average(&self) -> Duration {
        let total = self.0.iter().sum::<Duration>();
        total / (self.0.len() as u32)
    }

    fn min(&self) -> Duration {
        *self.0.iter().min().unwrap()
    }

    fn max(&self) -> Duration {
        *self.0.iter().max().unwrap()
    }

    fn variance(&self) -> Duration {
        let avg = self.average();
        let var = self
            .0
            .iter()
            .map(|d| {
                let diff = if *d > avg { *d - avg } else { avg - *d };
                diff.as_nanos().pow(2)
            })
            .sum::<u128>()
            / (self.0.len() as u128);
        Duration::from_nanos(var as u64)
    }

    fn stddev(&self) -> Duration {
        let var = self.variance();
        let stddev = (var.as_nanos() as f64).sqrt() as u64;
        Duration::from_nanos(stddev)
    }
}

struct Benchmarker {
    runs: usize,
    times: HashMap<String, (Box<dyn FnMut(&mut Vec<u32>)>, RunTime)>,
}

impl Benchmarker {
    fn new(runs: usize) -> Self {
        Self {
            runs,
            times: HashMap::new(),
        }
    }

    fn register<F>(&mut self, name: &str, func: F)
    where
        F: FnMut(&mut Vec<u32>) + 'static,
    {
        self.times
            .insert(name.to_string(), (Box::new(func), RunTime::new()));
    }

    fn run(&mut self, size: usize) {
        for _ in 0..self.runs {
            let data = build_vec(size);
            for (name, (func, runtime)) in self.times.iter_mut() {
                let mut data = data.clone();
                let start = std::time::Instant::now();

                func(&mut data);

                let duration = start.elapsed();
                runtime.add(duration);

                assert!(data.is_sorted(), "Data is not sorted correctly by {}", name);
            }
        }
    }

    fn report(&self) {
        for (name, (_, runtime)) in self.times.iter() {
            println!("Benchmark: {}", name);
            println!("  Runs: {}", self.runs);
            println!("  Average: {:?}", runtime.average());
            println!("  Min: {:?}", runtime.min());
            println!("  Max: {:?}", runtime.max());
            println!("  Stddev: {:?}", runtime.stddev());
            println!();
        }
    }
}

fn main() {
    // let mut benchmarker = Benchmarker::new(10);
    // benchmarker.register("radix sort", |data| radix_sort(data));
    // benchmarker.register("radix sort (swap)", |data| radix_sort_swap(data));
    // benchmarker.register("unstable sort", |data| data.sort_unstable());
    // // benchmarker.register("stable sort", |data| data.sort());

    // let size = 100_000;
    // println!("Running benchmarks with input size: {}", size);
    // benchmarker.run(size);
    // benchmarker.report();

    let mut values = vec![170, 45, 75, 90, 802, 24, 2, 66];

    println!("Unsorted values: {:?}", values);
    radix_sort(&mut values);

    println!("Sorted values: {:?}", values);
}
