use super::super::expand::{expand_adam7_bits, expand_adam7_bytes, subbyte_values};
use super::*;

#[test]
fn test_adam7() {
    /*
        1646
        7777
        5656
        7777
    */
    let it = Adam7Iterator::new(4, 4);
    let passes: Vec<_> = it.collect();
    assert_eq!(
        &*passes,
        &[
            Adam7Info {
                pass: 1,
                line: 0,
                samples: 1,
                width: 4,
            },
            Adam7Info {
                pass: 4,
                line: 0,
                samples: 1,
                width: 4,
            },
            Adam7Info {
                pass: 5,
                line: 0,
                samples: 2,
                width: 4,
            },
            Adam7Info {
                pass: 6,
                line: 0,
                samples: 2,
                width: 4,
            },
            Adam7Info {
                pass: 6,
                line: 1,
                samples: 2,
                width: 4,
            },
            Adam7Info {
                pass: 7,
                line: 0,
                samples: 4,
                width: 4,
            },
            Adam7Info {
                pass: 7,
                line: 1,
                samples: 4,
                width: 4,
            }
        ]
    );
}

#[test]
fn test_subbyte_pixels() {
    const BIT_POS_1: [u8; 8] = [7, 6, 5, 4, 3, 2, 1, 0];

    let scanline = &[0b10101010, 0b10101010];
    let pixels = subbyte_values(scanline, BIT_POS_1, 1).collect::<Vec<_>>();

    assert_eq!(pixels.len(), 16);
    assert_eq!(pixels, [1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0]);
}

#[test]
fn test_expand_adam7_bits() -> std::io::Result<()> {
    let width = 32;
    let bits_pp = 1;
    let stride = width / 8;
    let info = |pass, line, img_width| create_adam7_info_for_tests(pass, line as u32, img_width);

    let expected = |offset: usize, step: usize, count: usize| {
        (0..count)
            .map(move |i| step * i + offset)
            .map(|i| BitPostion {
                byte: i / 8,
                bit: (i % 8) as u8,
            })
            .collect::<Vec<_>>()
    };

    for line_no in 0..8 {
        let start = 8 * line_no * width;

        assert_eq!(
            expand_adam7_bits(stride, &info(1, line_no, width)?, bits_pp).collect::<Vec<_>>(),
            expected(start, 8, 4)
        );

        let start = start + 4;

        assert_eq!(
            expand_adam7_bits(stride, &info(2, line_no, width)?, bits_pp).collect::<Vec<_>>(),
            expected(start, 8, 4)
        );

        let start = (8 * line_no + 4) * width;

        assert_eq!(
            expand_adam7_bits(stride, &info(3, line_no, width)?, bits_pp).collect::<Vec<_>>(),
            expected(start, 4, 8)
        );
    }

    for line_no in 0..16 {
        let start = 4 * line_no * width + 2;

        assert_eq!(
            expand_adam7_bits(stride, &info(4, line_no, width)?, bits_pp).collect::<Vec<_>>(),
            expected(start, 4, 8)
        );

        let start = (4 * line_no + 2) * width;

        assert_eq!(
            expand_adam7_bits(stride, &info(5, line_no, width)?, bits_pp).collect::<Vec<_>>(),
            expected(start, 2, 16)
        )
    }

    for line_no in 0..32 {
        let start = 2 * line_no * width + 1;

        assert_eq!(
            expand_adam7_bits(stride, &info(6, line_no, width)?, bits_pp).collect::<Vec<_>>(),
            expected(start, 2, 16),
            "line_no: {}",
            line_no
        );

        let start = (2 * line_no + 1) * width;

        assert_eq!(
            expand_adam7_bits(stride, &info(7, line_no, width)?, bits_pp).collect::<Vec<_>>(),
            expected(start, 1, 32)
        );
    }
    Ok(())
}

#[test]
fn test_expand_adam7_bits_independent_row_stride() -> std::io::Result<()> {
    let pass = 1;
    let line_no = 1;
    let width = 32;
    let info = create_adam7_info_for_tests;

    {
        let stride = width;
        assert_eq!(
            expand_adam7_bytes(stride, &info(pass, line_no, width)?, 1).collect::<Vec<_>>(),
            [2048, 2112, 2176, 2240].map(|n| n / 8),
        );
    }

    {
        let stride = 10000;
        assert_eq!(
            expand_adam7_bytes(stride, &info(pass, line_no, width)?, 1).collect::<Vec<_>>(),
            [640000, 640064, 640128, 640192].map(|n| n / 8),
        );
    }
    Ok(())
}

#[test]
fn test_expand_pass_subbyte() -> std::io::Result<()> {
    let mut img = [0u8; 8];
    let width = 8;
    let stride = width / 8;
    let bits_pp = 1;
    let info = create_adam7_info_for_tests;

    expand_pass(
        &mut img,
        stride,
        &[0b10000000],
        &info(1, 0, width)?,
        bits_pp,
    );
    assert_eq!(img, [0b10000000u8, 0, 0, 0, 0, 0, 0, 0]);

    expand_pass(
        &mut img,
        stride,
        &[0b10000000],
        &info(2, 0, width)?,
        bits_pp,
    );
    assert_eq!(img, [0b10001000u8, 0, 0, 0, 0, 0, 0, 0]);

    expand_pass(
        &mut img,
        stride,
        &[0b11000000],
        &info(3, 0, width)?,
        bits_pp,
    );
    assert_eq!(img, [0b10001000u8, 0, 0, 0, 0b10001000, 0, 0, 0]);

    expand_pass(
        &mut img,
        stride,
        &[0b11000000],
        &info(4, 0, width)?,
        bits_pp,
    );
    assert_eq!(img, [0b10101010u8, 0, 0, 0, 0b10001000, 0, 0, 0]);

    expand_pass(
        &mut img,
        stride,
        &[0b11000000],
        &info(4, 1, width)?,
        bits_pp,
    );
    assert_eq!(img, [0b10101010u8, 0, 0, 0, 0b10101010, 0, 0, 0]);

    expand_pass(
        &mut img,
        stride,
        &[0b11110000],
        &info(5, 0, width)?,
        bits_pp,
    );
    assert_eq!(img, [0b10101010u8, 0, 0b10101010, 0, 0b10101010, 0, 0, 0]);

    expand_pass(
        &mut img,
        stride,
        &[0b11110000],
        &info(5, 1, width)?,
        bits_pp,
    );
    assert_eq!(
        img,
        [0b10101010u8, 0, 0b10101010, 0, 0b10101010, 0, 0b10101010, 0]
    );

    expand_pass(
        &mut img,
        stride,
        &[0b11110000],
        &info(6, 0, width)?,
        bits_pp,
    );
    assert_eq!(
        img,
        [0b11111111u8, 0, 0b10101010, 0, 0b10101010, 0, 0b10101010, 0]
    );

    expand_pass(
        &mut img,
        stride,
        &[0b11110000],
        &info(6, 1, width)?,
        bits_pp,
    );
    assert_eq!(
        img,
        [0b11111111u8, 0, 0b11111111, 0, 0b10101010, 0, 0b10101010, 0]
    );

    expand_pass(
        &mut img,
        stride,
        &[0b11110000],
        &info(6, 2, width)?,
        bits_pp,
    );
    assert_eq!(
        img,
        [0b11111111u8, 0, 0b11111111, 0, 0b11111111, 0, 0b10101010, 0]
    );

    expand_pass(
        &mut img,
        stride,
        &[0b11110000],
        &info(6, 3, width)?,
        bits_pp,
    );
    assert_eq!(
        [0b11111111u8, 0, 0b11111111, 0, 0b11111111, 0, 0b11111111, 0],
        img
    );

    expand_pass(
        &mut img,
        stride,
        &[0b11111111],
        &info(7, 0, width)?,
        bits_pp,
    );
    assert_eq!(
        [
            0b11111111u8,
            0b11111111,
            0b11111111,
            0,
            0b11111111,
            0,
            0b11111111,
            0
        ],
        img
    );

    expand_pass(
        &mut img,
        stride,
        &[0b11111111],
        &info(7, 1, width)?,
        bits_pp,
    );
    assert_eq!(
        [
            0b11111111u8,
            0b11111111,
            0b11111111,
            0b11111111,
            0b11111111,
            0,
            0b11111111,
            0
        ],
        img
    );

    expand_pass(
        &mut img,
        stride,
        &[0b11111111],
        &info(7, 2, width)?,
        bits_pp,
    );
    assert_eq!(
        [
            0b11111111u8,
            0b11111111,
            0b11111111,
            0b11111111,
            0b11111111,
            0b11111111,
            0b11111111,
            0
        ],
        img
    );

    expand_pass(
        &mut img,
        stride,
        &[0b11111111],
        &info(7, 3, width)?,
        bits_pp,
    );
    assert_eq!(
        [
            0b11111111u8,
            0b11111111,
            0b11111111,
            0b11111111,
            0b11111111,
            0b11111111,
            0b11111111,
            0b11111111
        ],
        img
    );
    Ok(())
}
