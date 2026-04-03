use std::{collections::HashMap, num::NonZeroU64, time::Duration};

use wgpu::{MemoryHints, PollType, util::DeviceExt};

const SHADER: &str = include_str!("radix_sort.wgsl");

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    n: u32,
    shift: u32,
    _pad: [u32; 2], // 16-byte align for uniform buffers
}

/// Straightforward CPU reference implementation that mirrors the logic from
/// `examples/radix_sort.rs`.  We keep it here to make comparing GPU vs CPU
/// behaviour self-contained for anyone studying this example.
fn radix_sort(input: &mut Vec<u32>) {
    if input.len() <= 1 {
        return;
    }

    let n = input.len();
    let mut out_buf = vec![0u32; n];
    let mut count = [0usize; 256];

    for shift in (0..32).step_by(8) {
        count.fill(0);
        for &x in input.iter() {
            count[((x >> shift) & 0xFF) as usize] += 1;
        }

        let mut sum = 0usize;
        for c in count.iter_mut() {
            let tmp = *c;
            *c = sum;
            sum += tmp;
        }

        for &x in input.iter() {
            let b = ((x >> shift) & 0xFF) as usize;
            let pos = count[b];
            out_buf[pos] = x;
            count[b] = pos + 1;
        }

        std::mem::swap(input, &mut out_buf);
    }
}

/// Helper that builds a random vector to feed the benchmarks.
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

/// Convenience wrapper that encapsulates the GPU state.  It lets us amortise
/// setup cost across multiple sorts, which makes the benchmark comparison with
/// the CPU version much more meaningful.
struct GpuRadixSorter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuRadixSorter {
    async fn new() -> Self {
        // Instance / Adapter / Device setup mirrors the original minimal
        // example; now we keep the objects around for many dispatches.
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapters found on the system!");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("radix::device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radix::compute_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radix::bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(NonZeroU64::new(4).unwrap()),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(NonZeroU64::new(4).unwrap()),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            NonZeroU64::new(std::mem::size_of::<Params>() as u64).unwrap(),
                        ),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("radix::pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("radix::compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
        }
    }

    fn sort(&mut self, values: &mut Vec<u32>) -> Result<(), String> {
        if values.is_empty() {
            return Ok(());
        }
        if values.len() > u32::MAX as usize {
            return Err(
                "Input length exceeds u32::MAX and cannot be represented on the GPU".to_string(),
            );
        }

        let len_u32 = values.len() as u32;

        let data_buf_a = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("radix::data_buf_a"),
                contents: bytemuck::cast_slice(values),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        let data_buf_b = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("radix::data_buf_b"),
            size: (values.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut params = Params {
            n: len_u32,
            shift: 0,
            _pad: [0; 2],
        };
        let params_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("radix::params_buf"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let readback_size = (values.len() * std::mem::size_of::<u32>()) as u64;
        let readback_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("radix::readback_buf"),
            size: readback_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let passes = [0u32, 8u32, 16u32, 24u32];
        for (pass_index, shift) in passes.into_iter().enumerate() {
            params.shift = shift;
            self.queue
                .write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));

            let (src_buf, dst_buf) = if pass_index % 2 == 0 {
                (&data_buf_a, &data_buf_b)
            } else {
                (&data_buf_b, &data_buf_a)
            };

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("radix::bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: src_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: dst_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buf.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("radix::pass_encoder"),
                });

            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("radix::compute_pass"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                // The shader streams over the entire array using a single workgroup for
                // clarity, so we dispatch just once.
                cpass.dispatch_workgroups(1, 1, 1);
            }

            self.queue.submit(Some(encoder.finish()));
            self.device
                .poll(PollType::Wait)
                .map_err(|_| "Failed to wait for GPU radix pass".to_string())?;
        }

        let final_src = if passes.len() % 2 == 0 {
            &data_buf_a
        } else {
            &data_buf_b
        };

        let mut copy_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("radix::copy_encoder"),
                });

        copy_encoder.copy_buffer_to_buffer(final_src, 0, &readback_buf, 0, readback_size);

        self.queue.submit(Some(copy_encoder.finish()));

        self.device
            .poll(PollType::Wait)
            .map_err(|_| "Failed to wait for GPU readback copy".to_string())?;

        let slice = readback_buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(PollType::Wait)
            .map_err(|_| "Failed to map readback buffer".to_string())?;

        {
            let data = slice.get_mapped_range();
            let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
            values.copy_from_slice(&result);
        }
        readback_buf.unmap();

        Ok(())
    }
}

fn main() {
    // Quick smoke test on the classic teaching dataset.
    let mut values = vec![170, 45, 75, 90, 802, 24, 2, 66];
    let mut target = values.clone();
    target.sort();

    let mut gpu_sorter = pollster::block_on(GpuRadixSorter::new());
    gpu_sorter
        .sort(&mut values)
        .expect("GPU radix sort failed on the smoke test");

    println!("Smoke test input : {:?}", target);
    println!("Smoke test result: {:?}", values);
    assert_eq!(values, target);

    // Additional sanity check on random data to aid development debugging.
    let sample = build_vec(128);
    let mut cpu_sorted = sample.clone();
    radix_sort(&mut cpu_sorted);
    let mut gpu_sorted = sample.clone();
    gpu_sorter
        .sort(&mut gpu_sorted)
        .expect("GPU radix sort failed on random sample");
    if cpu_sorted != gpu_sorted {
        eprintln!(
            "CPU sorted (first 32): {:?}",
            &cpu_sorted[..32.min(cpu_sorted.len())]
        );
        eprintln!(
            "GPU sorted (first 32): {:?}",
            &gpu_sorted[..32.min(gpu_sorted.len())]
        );
        panic!("GPU output diverged from CPU reference on random sample");
    }

    // Development sanity check on a much larger array to catch edge cases.
    let large_check_size = 200_000;
    let large_sample = build_vec(large_check_size);
    let mut cpu_large = large_sample.clone();
    radix_sort(&mut cpu_large);
    let mut gpu_large = large_sample.clone();
    gpu_sorter
        .sort(&mut gpu_large)
        .expect("GPU radix sort failed on large sample");
    if cpu_large != gpu_large {
        for i in 0..cpu_large.len() {
            if cpu_large[i] != gpu_large[i] {
                eprintln!(
                    "Mismatch at index {}: cpu={} gpu={}",
                    i, cpu_large[i], gpu_large[i]
                );
                if i > 64 {
                    break;
                }
            }
        }
        panic!("GPU output diverged from CPU reference on large sample");
    }

    // Benchmark CPU vs GPU using the shared `Benchmarker` harness.
    let mut benchmarker = Benchmarker::new(5);
    benchmarker.register("CPU radix sort", |data| radix_sort(data));

    benchmarker.register("GPU radix sort", {
        let mut sorter = gpu_sorter;
        move |data| {
            sorter
                .sort(data)
                .expect("GPU radix sort failed during benchmarking");
        }
    });

    benchmarker.register("std sort_unstable", |data| data.sort_unstable());

    let size = 1_000;
    println!(
        "Running benchmarks with input size {}, {} runs each",
        size, benchmarker.runs
    );
    benchmarker.run(size);
    benchmarker.report();
}
