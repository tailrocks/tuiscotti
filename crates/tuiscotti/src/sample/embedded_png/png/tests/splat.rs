use super::super::expand::{expand_adam7_bits, subbyte_values};
use super::*;

#[test]
fn test_expand_pass_splat_4bpp() -> std::io::Result<()> {
    let width = 8;
    let bits_pp = 4;

    let mut img = [0u8; 8];
    let stride = width / 2;

    let passes: &[(&'static [u8], &'static [u8])] = &[
        (&[0x10], &[0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11]), // pass 1, 0
        (&[0x20], &[0x11, 0x11, 0x22, 0x22, 0x11, 0x11, 0x22, 0x22]), // pass 2, 0
        // no third pass..
        (&[0x4a], &[0x11, 0x44, 0x22, 0xaa, 0x11, 0x44, 0x22, 0xaa]), // pass 4, 0
        // no fifth pass..
        (
            &[0x6b, 0x6b],
            &[0x16, 0x4b, 0x26, 0xab, 0x16, 0x4b, 0x26, 0xab],
        ), // pass 6, 0
        (
            &[0x7c, 0xc7, 0x7c, 0x7c],
            &[0x16, 0x4b, 0x26, 0xab, 0x7c, 0xc7, 0x7c, 0x7c],
        ), // pass 7, 0
    ];

    let adam7 = Adam7Iterator::new(8, 2);
    for ((data, expected), adam7_info) in passes.iter().zip(adam7) {
        expand_pass_splat(&mut img, stride, data, &adam7_info, bits_pp)?;
        assert_eq!(img, *expected, "{img:x?} {expected:x?} {adam7_info:?}");
    }
    Ok(())
}

#[test]
fn adam7_equivalence() -> std::io::Result<()> {
    // Choose ragged sizes to cover bugs that write outside etc.
    const WIDTH: u32 = 8;
    const HEIGHT: u32 = 8;

    let interace_pool: Vec<_> = (0x42u8..).take(32).collect();

    for &bpp in &[1u8, 2, 4, 8, 16, 24, 32][2..] {
        let bytes_of = |pix: u32| (u32::from(bpp) * pix).next_multiple_of(8) as usize / 8;

        let rowbytes = bytes_of(WIDTH);

        // In the sparse case we do not promise to override all bits
        let mut buf_sparse = vec![0x00; rowbytes * HEIGHT as usize];
        // Whereas in the spat case we do, so we may as well set some arbitrary initial
        let mut buf_splat = vec![0xaa; rowbytes * HEIGHT as usize];

        // Now execute all the iterations, then compare buffers.
        for adam7_info in Adam7Iterator::new(WIDTH, HEIGHT) {
            let adam7_bytes = bytes_of(adam7_info.samples);
            let interlace_line = &interace_pool[..adam7_bytes];

            expand_pass(&mut buf_sparse, rowbytes, interlace_line, &adam7_info, bpp);
            expand_pass_splat(&mut buf_splat, rowbytes, interlace_line, &adam7_info, bpp)?;
        }

        assert_eq!(
            buf_sparse, buf_splat,
            "{buf_sparse:x?} {buf_splat:x?} bpp={bpp}"
        );
    }
    Ok(())
}

#[test]
fn test_expand_pass_splat_1bpp() -> std::io::Result<()> {
    let width = 8;
    let bits_pp = 1;

    let mut img = [0u8; 8];
    let stride = 1;

    // Since bits do not suffice to represent the pass number in pixels we choose interlace
    // rows such that we toggle all affected bits each time. In particular the input bits that
    // must not be used are set to the inverse.
    let passes: &[(&'static [u8], &'static [u8])] = &[
        (&[0x80], &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]), // pass 1, 0
        (&[0x7f], &[0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0]), // pass 2, 0
        (&[0xc0], &[0xf0, 0xf0, 0xf0, 0xf0, 0xff, 0xff, 0xff, 0xff]), // pass 3, 0
        (&[0x3f], &[0xc0, 0xc0, 0xc0, 0xc0, 0xff, 0xff, 0xff, 0xff]), // pass 4, 0
        (&[0x3f], &[0xc0, 0xc0, 0xc0, 0xc0, 0xcc, 0xcc, 0xcc, 0xcc]), // pass 4, 1
        (&[0xf0], &[0xc0, 0xc0, 0xff, 0xff, 0xcc, 0xcc, 0xcc, 0xcc]), // pass 5, 0
        (&[0xf0], &[0xc0, 0xc0, 0xff, 0xff, 0xcc, 0xcc, 0xff, 0xff]), // pass 5, 1
        (&[0x0f], &[0x80, 0x80, 0xff, 0xff, 0xcc, 0xcc, 0xff, 0xff]), // pass 6, 0
        (&[0x0f], &[0x80, 0x80, 0xaa, 0xaa, 0xcc, 0xcc, 0xff, 0xff]), // pass 6, 1
        (&[0x0f], &[0x80, 0x80, 0xaa, 0xaa, 0x88, 0x88, 0xff, 0xff]), // pass 6, 2
        (&[0x0f], &[0x80, 0x80, 0xaa, 0xaa, 0x88, 0x88, 0xaa, 0xaa]), // pass 6, 3
        (&[0xff], &[0x80, 0xff, 0xaa, 0xaa, 0x88, 0x88, 0xaa, 0xaa]), // pass 7, 0
        (&[0xff], &[0x80, 0xff, 0xaa, 0xff, 0x88, 0x88, 0xaa, 0xaa]), // pass 7, 1
        (&[0xff], &[0x80, 0xff, 0xaa, 0xff, 0x88, 0xff, 0xaa, 0xaa]), // pass 7, 2
        (&[0xff], &[0x80, 0xff, 0xaa, 0xff, 0x88, 0xff, 0xaa, 0xff]), // pass 7, 3
    ];

    let adam7 = Adam7Iterator::new(width, 8);
    for ((data, expected), adam7_info) in passes.iter().zip(adam7) {
        expand_pass_splat(&mut img, stride, data, &adam7_info, bits_pp)?;
        assert_eq!(img, *expected, "{img:x?} {expected:x?} {adam7_info:?}");
    }
    Ok(())
}

#[test]
fn regression_overflow_adam7_bitfill() -> std::io::Result<()> {
    fn multibyte_expand_pass_test_helper(width: usize, height: usize, bits_pp: u8) -> Vec<u8> {
        let bytes_pp = bits_pp / 8;
        let size = width * height * bytes_pp as usize;
        let mut img = vec![0u8; size];
        let img_row_stride = width * bytes_pp as usize;

        for it in Adam7Iterator::new(width as u32, height as u32).into_iter() {
            if it.pass != 7 {
                continue;
            }

            if it.line != (width / 2) as u32 - 1 {
                continue;
            }

            let interlace_size = it.width * (bytes_pp as u32);
            // Ensure that expanded pixels are never empty bits. This differentiates the written bits
            // from the initial bits that are all zeroed.
            let interlaced_row: Vec<_> = (0..interlace_size).map(|_| 0xff).collect();

            expand_pass(&mut img, img_row_stride, &interlaced_row, &it, bits_pp);
        }

        img
    }

    let expanded = multibyte_expand_pass_test_helper(1 << 14, 1 << 14, 32);
    let last = expanded
        .last()
        .ok_or_else(|| std::io::Error::other("expanded image is empty"))?;
    assert_eq!(*last, 0xff);
    Ok(())
}
