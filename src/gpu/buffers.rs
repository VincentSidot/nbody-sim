use bytemuck::cast_slice;

use crate::sim::{SimParams, SimUniform};

pub struct GpuBuffers {
    /// Buffer containing particle positions (primary)
    pub positions_primary: wgpu::Buffer,
    /// Buffer containing particle positions (secondary)
    pub positions_secondary: wgpu::Buffer,
    /// Buffer containing particle velocities (primary)
    pub velocities_primary: wgpu::Buffer,
    /// Buffer containing particle velocities (secondary)
    pub velocities_secondary: wgpu::Buffer,
    /// Buffer containing particle colors
    pub colors: wgpu::Buffer,
    /// Buffer containing simulation parameters
    pub uniform: wgpu::Buffer,
    /// Number of particles the buffers can hold
    pub capacity: u32,
}

impl GpuBuffers {
    pub fn resize(&mut self, device: &wgpu::Device, new_capacity: u32) {
        if new_capacity <= self.capacity {
            return;
        }
        *self = Self::create(device, new_capacity);
    }

    pub fn upload_data(
        &self,
        queue: &wgpu::Queue,
        positions: Option<&[[f32; 2]]>,
        velocities: Option<&[[f32; 2]]>,
        colors: Option<&[[f32; 4]]>,
        uniform: Option<&SimParams>,
    ) {
        if let Some(positions) = positions {
            queue.write_buffer(&self.positions_primary, 0, cast_slice(positions));
            // Push here to avoid display issues on first frame
            queue.write_buffer(&self.positions_secondary, 0, cast_slice(positions));
        }
        if let Some(velocities) = velocities {
            queue.write_buffer(&self.velocities_primary, 0, cast_slice(velocities));
        }
        if let Some(colors) = colors {
            queue.write_buffer(&self.colors, 0, cast_slice(colors));
        }
        if let Some(params) = uniform {
            let uniform = params.to_uniform();
            queue.write_buffer(&self.uniform, 0, cast_slice(std::slice::from_ref(&uniform)));
        }
    }

    pub fn create(device: &wgpu::Device, mut capacity: u32) -> Self {
        // Align capacity to the closest power of two for better memory alignment
        capacity = capacity.next_power_of_two();

        let f2_size = std::mem::size_of::<[f32; 2]>() as u64;
        let f4_size = std::mem::size_of::<[f32; 4]>() as u64;

        let pos_size = f2_size * capacity as u64;
        let vel_size = f2_size * capacity as u64;
        let col_size = f4_size * capacity as u64;

        let mk = |label: &str, size: u64, usage: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };

        let positions_primary = mk(
            "positions_primary",
            pos_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        let positions_secondary = mk(
            "positions_secondary",
            pos_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        let velocities_primary = mk(
            "velocities",
            vel_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        let velocities_secondary = mk(
            "velocities_secondary",
            vel_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        let colors = mk(
            "colors_primary",
            col_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        let uniform = mk(
            "sim_params",
            std::mem::size_of::<SimUniform>() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        Self {
            positions_primary,
            positions_secondary,
            velocities_primary,
            velocities_secondary,
            colors,
            uniform,
            capacity,
        }
    }
}
