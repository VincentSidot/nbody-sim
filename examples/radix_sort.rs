//! Radix sort implementation for investigating GPU sorting algorithms.
//! Should be replaced with a wgsl version in the future.

pub fn radix_sort(mut input: &mut [u32]) {
    if input.len() <= 1 {
        return;
    }

    let n = input.len();
    let mut scratch = vec![0u32; n];

    // Work on two mutable slice bindings we can swap safely.
    let mut out_buf: &mut [u32] = &mut scratch;

    // 4 byte-wise passes: 0, 8, 16, 24
    for shift in (0..32).step_by(8) {
        // Counting phase
        let mut count = [0usize; 256];
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
        for &x in input.iter() {
            let b = ((x >> shift) & 0xFF) as usize;
            let pos = count[b];
            out_buf[pos] = x;
            count[b] = pos + 1;
        }

        // Next pass: swap which buffer is read vs written
        std::mem::swap(&mut input, &mut out_buf);
    }

    // If we ended up in the scratch buffer, copy back
    if input.as_ptr() != input.as_ptr() {
        input.copy_from_slice(input);
    }
}

fn main() {
    println!("Hello, world!");
}
