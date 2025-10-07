use crate::{constants, gpu::buffers::GpuBuffers};

pub fn make_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    let shader_str = include_str!("../../shaders/nbody.wgsl").replace(
        constants::shader::WORKGROUP_SIZE_PAYLOAD,
        &constants::shader::WORKGROUP_SIZE.to_string(),
    );

    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("compute_shader"),
        source: wgpu::ShaderSource::Wgsl(shader_str.into()),
    })
}

pub fn make_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("compute_pipeline_layout"),
        bind_group_layouts,
        push_constant_ranges: &[],
    })
}

pub fn make_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("compute_pipeline"),
        layout: Some(pipeline_layout),
        module: shader,
        entry_point: Some("update"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None, // No pipeline cache
    })
}

pub fn make_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("compute_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                // positions write (read-write)
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                // velocities write (read-write)
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                // positions read (read-only)
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                // velocities read (read-only)
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                // color
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                // uniform
                binding: 5,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// Make two bind groups, one for each buffer set (primary and secondary)
///
/// ID0 := write primary, read secondary
/// ID1 := write secondary, read primary
pub fn make_bind_group(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    buffers: &GpuBuffers,
) -> [wgpu::BindGroup; 2] {
    [
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_bg_primary"),
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    // positions primary
                    binding: 0,
                    resource: buffers.positions_primary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // velocities primary
                    binding: 1,
                    resource: buffers.velocities_primary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // positions secondary
                    binding: 2,
                    resource: buffers.positions_secondary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // velocities secondary
                    binding: 3,
                    resource: buffers.velocities_secondary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // color
                    binding: 4,
                    resource: buffers.colors.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // uniform
                    binding: 5,
                    resource: buffers.uniform.as_entire_binding(),
                },
            ],
        }),
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_bg_primary"),
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    // positions secondary
                    binding: 0,
                    resource: buffers.positions_secondary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // velocities secondary
                    binding: 1,
                    resource: buffers.velocities_secondary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // positions primary
                    binding: 2,
                    resource: buffers.positions_primary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // velocities primary
                    binding: 3,
                    resource: buffers.velocities_primary.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // color
                    binding: 4,
                    resource: buffers.colors.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    // uniform
                    binding: 5,
                    resource: buffers.uniform.as_entire_binding(),
                },
            ],
        }),
    ]
}
