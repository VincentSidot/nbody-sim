# Particle Playground

A high-performance particle simulation playground built with Rust and WebGPU (wgpu), featuring real-time N-body gravitational physics simulation with interactive controls.

## 🌌 Features

- **Real-time N-body Simulation**: Simulate thousands to millions of particles interacting via gravitational forces
- **GPU-Accelerated**: Computation and rendering performed entirely on the GPU using compute shaders
- **Interactive Controls**: Adjust simulation parameters in real-time via an intuitive UI
- **Visual Customization**: Toggle color-by-speed visualization and world wrapping
- **Performance Metrics**: Real-time frame rate and timing information
- **Configurable Parameters**: Adjust time step, gravitational constant, damping, and more

## 🛠️ Built With

- **Rust**: Systems programming language for maximum performance and memory safety
- **wgpu**: Rust implementation of WebGPU for cross-platform graphics and compute
- **winit**: Cross-platform window and event handling
- **egui**: Immediate mode GUI for real-time controls
- **glam**: Linear algebra library for vector mathematics
- **WGSL Shaders**: Compute shaders for efficient particle physics calculations

## 📋 Simulation Parameters

- **Time Step (dt)**: Controls the simulation time increment per frame
- **Gravitational Constant (g)**: Strength of gravitational attraction
- **Softening Factor**: Prevents singularities when particles get too close
- **Damping Factor**: Controls velocity decay over time
- **Particle Count**: Adjustable from 1 to 1,000,000 particles
- **World Wrapping**: Particles reappear on opposite side when crossing boundaries
- **Color by Speed**: Visualize particle velocity through color mapping

## 🚀 Getting Started

### Prerequisites

- Rust (latest stable)
- Cargo
- Compatible GPU with WebGPU support (Vulkan, Metal, or DX12)

### Building

```bash
# Clone the repository
git clone <repository-url>
cd particle-playground

# Build the project
cargo build --release

# Run the simulation
cargo run
```

## 🎮 Controls

- **Reset Particles**: Generate a new galaxy with default parameters
- **Pause/Resume**: Toggle simulation execution
- **Step**: Advance simulation by one frame (when paused)
- **Real-time Sliders**: Adjust all parameters while simulation runs
- **Fullscreen**: Use OS-native window controls for fullscreen mode

## 🏗️ Architecture

The project is structured around:

- **GPU Compute Pipeline**: N-body physics computed with custom WGSL shaders
- **Double Buffering**: Alternating between primary and secondary buffers for smooth simulation
- **Egui Integration**: Real-time UI overlaid on the particle visualization
- **Modular Design**: Separate modules for GPU operations, simulation parameters, and utilities

### Key Components

- `src/gpu/`: GPU state management, compute pipelines, and buffer management
- `src/sim/`: Simulation logic, parameters, and particle initialization
- `src/app.rs`: Main application state and event handling
- `shaders/nbody.wgsl`: Core N-body physics compute shader
- `shaders/render.wgsl`: Particle rendering vertex/fragment shader

## 📊 Performance

The simulation efficiently handles large numbers of particles using GPU compute shaders:
- 100,000+ particles at interactive frame rates
- Double buffering to prevent read/write conflicts
- Workgroup-optimized compute shaders with configurable sizes
- Memory-aligned particle buffers for optimal GPU access

## 🔮 Future Enhancements

- Expand particle modelisations (e.g., mass, collisions, etc.)
- Move into a 3D simulation space (needs research)
- Add more interactive controls (e.g., click to add particles, drag to create forces, etc.)
- Implement spatial partitioning for improved performance with very large particle counts
- Implement particle mesh solvers for larger scale simulations (needs research and maybe fft transforms on gpu side wtf?)
- Implement more complex initial conditions (e.g., spiral galaxies, clusters)
- Additional force models (electromagnetic, spring, etc.)
- Better particle initialization patterns
- Export/record functionality
- Additional visualization modes