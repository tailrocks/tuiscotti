use super::*;

fn filter_internal(
    method: RowFilter,
    bpp: usize,
    len: usize,
    previous: &[u8],
    current: &[u8],
    output: &mut [u8],
) -> RowFilter {
    use self::RowFilter::*;

    // This value was chosen experimentally based on what achieved the best performance. The
    // Rust compiler does auto-vectorization, and 32-bytes per loop iteration seems to enable
    // the fastest code when doing so.
    const CHUNK_SIZE: usize = 32;

    match method {
        NoFilter => {
            output.copy_from_slice(current);
            NoFilter
        }
        Sub => {
            let mut out_chunks = output[bpp..].chunks_exact_mut(CHUNK_SIZE);
            let mut cur_chunks = current[bpp..].chunks_exact(CHUNK_SIZE);
            let mut prev_chunks = current[..len - bpp].chunks_exact(CHUNK_SIZE);

            for ((out, cur), prev) in (&mut out_chunks).zip(&mut cur_chunks).zip(&mut prev_chunks) {
                for i in 0..CHUNK_SIZE {
                    out[i] = cur[i].wrapping_sub(prev[i]);
                }
            }

            for ((out, cur), &prev) in out_chunks
                .into_remainder()
                .iter_mut()
                .zip(cur_chunks.remainder())
                .zip(prev_chunks.remainder())
            {
                *out = cur.wrapping_sub(prev);
            }

            output[..bpp].copy_from_slice(&current[..bpp]);
            Sub
        }
        Up => {
            let mut out_chunks = output.chunks_exact_mut(CHUNK_SIZE);
            let mut cur_chunks = current.chunks_exact(CHUNK_SIZE);
            let mut prev_chunks = previous.chunks_exact(CHUNK_SIZE);

            for ((out, cur), prev) in (&mut out_chunks).zip(&mut cur_chunks).zip(&mut prev_chunks) {
                for i in 0..CHUNK_SIZE {
                    out[i] = cur[i].wrapping_sub(prev[i]);
                }
            }

            for ((out, cur), &prev) in out_chunks
                .into_remainder()
                .iter_mut()
                .zip(cur_chunks.remainder())
                .zip(prev_chunks.remainder())
            {
                *out = cur.wrapping_sub(prev);
            }
            Up
        }
        Avg => {
            let mut out_chunks = output[bpp..].chunks_exact_mut(CHUNK_SIZE);
            let mut cur_chunks = current[bpp..].chunks_exact(CHUNK_SIZE);
            let mut cur_minus_bpp_chunks = current[..len - bpp].chunks_exact(CHUNK_SIZE);
            let mut prev_chunks = previous[bpp..].chunks_exact(CHUNK_SIZE);

            for (((out, cur), cur_minus_bpp), prev) in (&mut out_chunks)
                .zip(&mut cur_chunks)
                .zip(&mut cur_minus_bpp_chunks)
                .zip(&mut prev_chunks)
            {
                for i in 0..CHUNK_SIZE {
                    // Bitwise average of two integers without overflow and
                    // without converting to a wider bit-width. See:
                    // http://aggregate.org/MAGIC/#Average%20of%20Integers
                    // If this is unrolled by component, consider reverting to
                    // `((cur_minus_bpp[i] as u16 + prev[i] as u16) / 2) as u8`
                    out[i] = cur[i].wrapping_sub(
                        (cur_minus_bpp[i] & prev[i]) + ((cur_minus_bpp[i] ^ prev[i]) >> 1),
                    );
                }
            }

            for (((out, cur), &cur_minus_bpp), &prev) in out_chunks
                .into_remainder()
                .iter_mut()
                .zip(cur_chunks.remainder())
                .zip(cur_minus_bpp_chunks.remainder())
                .zip(prev_chunks.remainder())
            {
                *out = cur.wrapping_sub((cur_minus_bpp & prev) + ((cur_minus_bpp ^ prev) >> 1));
            }

            for i in 0..bpp {
                output[i] = current[i].wrapping_sub(previous[i] / 2);
            }
            Avg
        }
        Paeth => {
            let mut out_chunks = output[bpp..].chunks_exact_mut(CHUNK_SIZE);
            let mut cur_chunks = current[bpp..].chunks_exact(CHUNK_SIZE);
            let mut a_chunks = current[..len - bpp].chunks_exact(CHUNK_SIZE);
            let mut b_chunks = previous[bpp..].chunks_exact(CHUNK_SIZE);
            let mut c_chunks = previous[..len - bpp].chunks_exact(CHUNK_SIZE);

            for ((((out, cur), a), b), c) in (&mut out_chunks)
                .zip(&mut cur_chunks)
                .zip(&mut a_chunks)
                .zip(&mut b_chunks)
                .zip(&mut c_chunks)
            {
                for i in 0..CHUNK_SIZE {
                    out[i] = cur[i].wrapping_sub(paeth::filter_paeth_fpnge(a[i], b[i], c[i]));
                }
            }

            for ((((out, cur), &a), &b), &c) in out_chunks
                .into_remainder()
                .iter_mut()
                .zip(cur_chunks.remainder())
                .zip(a_chunks.remainder())
                .zip(b_chunks.remainder())
                .zip(c_chunks.remainder())
            {
                *out = cur.wrapping_sub(paeth::filter_paeth_fpnge(a, b, c));
            }

            for i in 0..bpp {
                output[i] = current[i].wrapping_sub(paeth::filter_paeth_fpnge(0, previous[i], 0));
            }
            Paeth
        }
    }
}

fn adaptive_filter(
    f: impl Fn(&[u8]) -> u64,
    bpp: usize,
    len: usize,
    previous: &[u8],
    current: &[u8],
    output: &mut [u8],
) -> RowFilter {
    use RowFilter::*;

    let mut min_cost: u64 = u64::MAX;
    let mut filter_choice = RowFilter::NoFilter;
    for &filter in [Up, Sub, Avg, Paeth].iter() {
        filter_internal(filter, bpp, len, previous, current, output);
        let cost = f(output);
        if cost <= min_cost {
            min_cost = cost;
            filter_choice = filter;

            if cost == 0 {
                return filter_choice;
            }
        }
    }
    if filter_choice != Paeth {
        filter_internal(filter_choice, bpp, len, previous, current, output);
    }
    filter_choice
}

pub(crate) fn filter(
    method: Filter,
    bpp: BytesPerPixel,
    previous: &[u8],
    current: &[u8],
    output: &mut [u8],
) -> RowFilter {
    let bpp = bpp.into_usize();
    let len = current.len();

    match method {
        Filter::Adaptive => adaptive_filter(sum_buffer, bpp, len, previous, current, output),
        Filter::MinEntropy => adaptive_filter(entropy, bpp, len, previous, current, output),
        Filter::NoFilter => {
            filter_internal(RowFilter::NoFilter, bpp, len, previous, current, output)
        }
        Filter::Sub => filter_internal(RowFilter::Sub, bpp, len, previous, current, output),
        Filter::Up => filter_internal(RowFilter::Up, bpp, len, previous, current, output),
        Filter::Avg => filter_internal(RowFilter::Avg, bpp, len, previous, current, output),
        Filter::Paeth => filter_internal(RowFilter::Paeth, bpp, len, previous, current, output),
    }
}

/// Estimate the value of i * log2(i) without using floating point operations,
/// implementation originally from oxipng.
fn ilog2i(i: u32) -> u32 {
    let log = 32 - i.leading_zeros() - 1;
    i * log + ((i - (1 << log)) << 1)
}

fn entropy(buf: &[u8]) -> u64 {
    let mut counts = [[0_u32; 256]; 4];
    let mut total = 0;

    // Count the number of occurrences of each byte value.
    let mut chunks = buf.chunks_exact(8);
    for chunk in &mut chunks {
        // Runs of zeros are common and very compressible, so treat them as free.
        if chunk == [0; 8] {
            continue;
        }

        // Scatter the counts into 4 separate arrays to reduce contention.
        for j in 0..2 {
            counts[0][chunk[j * 4] as usize] += 1;
            counts[1][chunk[1 + j * 4] as usize] += 1;
            counts[2][chunk[2 + j * 4] as usize] += 1;
            counts[3][chunk[3 + j * 4] as usize] += 1;
        }
        total += 8;
    }
    for &lit in chunks.remainder() {
        counts[0][lit as usize] += 1;
        total += 1;
    }

    // If the input is entirely zeros, short-circuit the entropy calculation.
    if counts[0][0] == total {
        return 0;
    }

    // Consolidate the counts.
    //
    let (first, remaining) = counts.split_at_mut(1);
    let (green, remaining) = remaining.split_at_mut(1);
    let (blue, alpha) = remaining.split_at_mut(1);
    for (((count, green), blue), alpha) in first[0]
        .iter_mut()
        .zip(&green[0])
        .zip(&blue[0])
        .zip(&alpha[0])
    {
        *count += green + blue + alpha;
    }

    // Compute the entropy.
    let mut entropy = ilog2i(total);
    for &count in &counts[0] {
        if count > 0 {
            entropy = entropy.saturating_sub(ilog2i(count));
        }
    }

    entropy as u64
}

// Helper function for Adaptive filter buffer summation
pub(crate) fn sum_buffer(buf: &[u8]) -> u64 {
    const CHUNK_SIZE: usize = 32;

    let mut buf_chunks = buf.chunks_exact(CHUNK_SIZE);
    let mut sum = 0_u64;

    for chunk in &mut buf_chunks {
        // At most, `acc` can be `32 * (i8::MIN as u8) = 32 * 128 = 4096`.
        let mut acc = 0;
        for &b in chunk {
            acc += u64::from((b as i8).unsigned_abs());
        }
        sum = sum.saturating_add(acc);
    }

    let mut acc = 0;
    for &b in buf_chunks.remainder() {
        acc += u64::from((b as i8).unsigned_abs());
    }

    sum.saturating_add(acc)
}
