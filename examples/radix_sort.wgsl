struct Params {
    n: u32,
    shift: u32,
    _pad0: u32,
    _pad1: u32,
};

// --- Tunables --------------------------------------------------------------
// These constants keep the shader readable and make the learning experience
// approachable.  Feel free to experiment with them once the baseline code is
// clear.
const WORKGROUP_SIZE: u32 = 64u;
const RADIX: u32 = 256u;         // 8 bits per pass => 4 passes for u32
const RADIX_MASK: u32 = RADIX - 1u;

@group(0) @binding(0) var<storage, read> src_data: array<u32>;
@group(0) @binding(1) var<storage, read_write> dst_data: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

// Shared-memory helpers used during the two-phase pass (counting + scatter).
var<workgroup> bucket_counts: array<atomic<u32>, RADIX>;
var<workgroup> bucket_offsets: array<u32, RADIX>;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(workgroup_id) workgroup: vec3<u32>,
    @builtin(local_invocation_id) local_id_vec: vec3<u32>,
) {
    let n = params.n;
    let local_id = local_id_vec.x;

    // This pedagogical shader operates with a single workgroup that streams
    // across the entire data set.  That keeps the logic easy to follow while
    // still handling arbitrarily large inputs.
    if (workgroup.x > 0u || workgroup.y > 0u || workgroup.z > 0u) {
        return;
    }
    if (n == 0u) {
        return;
    }

    // ----------------------------------------------------------------------
    // Counting phase – walk through the source data in strides of
    // `WORKGROUP_SIZE`, accumulating per-digit frequencies.
    for (var bucket = local_id; bucket < RADIX; bucket = bucket + WORKGROUP_SIZE) {
        atomicStore(&bucket_counts[bucket], 0u);
    }
    workgroupBarrier();

    for (var idx = local_id; idx < n; idx = idx + WORKGROUP_SIZE) {
        let value = src_data[idx];
        let digit = (value >> params.shift) & RADIX_MASK;
        atomicAdd(&bucket_counts[digit], 1u);
    }
    workgroupBarrier();

    // ----------------------------------------------------------------------
    // Exclusive prefix sums – convert frequencies into absolute starting
    // offsets inside the destination buffer.  A single lane is plenty here.
    if (local_id == 0u) {
        var running_total: u32 = 0u;
        for (var bucket = 0u; bucket < RADIX; bucket = bucket + 1u) {
            let count = atomicLoad(&bucket_counts[bucket]);
            bucket_offsets[bucket] = running_total;
            running_total = running_total + count;
        }
    }
    workgroupBarrier();

    // ----------------------------------------------------------------------
    // Scatter phase – stream the source a second time, writing each element
    // into `dst_data`.  The inner loop processes the staged tile in invocation
    // order to maintain stability (crucial for LSD radix sort).
    if (local_id == 0u) {
        for (var idx = 0u; idx < n; idx = idx + 1u) {
            let value = src_data[idx];
            let digit = (value >> params.shift) & RADIX_MASK;
            let write_index = bucket_offsets[digit];
            bucket_offsets[digit] = write_index + 1u;
            dst_data[write_index] = value;
        }
    }
    workgroupBarrier();
}
