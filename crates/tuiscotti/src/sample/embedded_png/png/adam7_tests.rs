use super::*;

fn create_adam7_info_for_tests(
    pass: u8,
    line: u32,
    img_width: usize,
) -> std::io::Result<Adam7Info> {
    let width = {
        let img_height = 8;
        Adam7Iterator::new(img_width as u32, img_height)
            .filter(|info| info.pass == pass)
            .map(|info| info.samples)
            .next()
            .ok_or_else(|| std::io::Error::other("Adam7 pass has no samples"))?
    };

    Ok(Adam7Info {
        pass,
        line,
        samples: width,
        width,
    })
}

#[path = "tests/basic.rs"]
mod basic;
#[path = "tests/splat.rs"]
mod splat;
