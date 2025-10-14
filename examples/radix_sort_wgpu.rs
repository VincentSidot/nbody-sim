use std::num::NonZeroU64;
use wgpu::{MemoryHints, PollType, util::DeviceExt};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    n: u32,
    _pad: [u32; 3], // 16-byte align for uniform buffers
}

const SHADER: &str = include_str!("radix_sort.wgsl");

fn main() {
    // Input data
    let input: Vec<u32> = vec![170, 45, 75, 90, 802, 24, 2, 66];
    let target_input = {
        let mut v = input.clone();
        v.sort();
        v
    };
    let n = input.len() as u32;

    // Run everything on the current thread
    pollster::block_on(async {
        // 1) Instance / Adapter / Device
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
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        // 2) Buffers
        let data_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("data_buf (storage)"),
            contents: bytemuck::cast_slice(&input),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let params = Params { n, _pad: [0; 3] };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params_buf (uniform)"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Buffer to read results back on CPU
        let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_buf (MAP_READ)"),
            size: (input.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 3) Shader & Pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind_group_layout"),
            entries: &[
                // storage buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(NonZeroU64::new(4).unwrap()), // at least one u32
                    },
                    count: None,
                },
                // uniform (count)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind_group"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: data_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // 4) Encode & Dispatch
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("encoder"),
        });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            let wg_size = 64u32;
            let groups = ((n + wg_size - 1) / wg_size).max(1);
            cpass.dispatch_workgroups(groups, 1, 1);
        }

        // Copy GPU data -> readback buffer
        encoder.copy_buffer_to_buffer(
            &data_buf,
            0,
            &readback_buf,
            0,
            (input.len() * std::mem::size_of::<u32>()) as u64,
        );

        // Submit work
        queue.submit(Some(encoder.finish()));

        // 5) Read back
        // Ensure all work is done before mapping
        device
            .poll(PollType::Wait)
            .expect("Failed wait for GPU operations");

        let slice = readback_buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(PollType::Wait)
            .expect("Failed wait for buffer mapping");

        let data = slice.get_mapped_range();
        let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        readback_buf.unmap();

        println!("input : {:?}", input);
        println!("result: {:?}", result);
        println!("target: {:?}", target_input);

        assert_eq!(result, target_input);
        println!("Success!");
    });
}
