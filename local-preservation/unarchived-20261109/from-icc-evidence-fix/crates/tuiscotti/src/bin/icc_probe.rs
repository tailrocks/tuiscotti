use image::ImageDecoder as _;
use std::{env, fs, io::Cursor, process};

fn main() {
    let mut args = env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| { eprintln!("usage: icc_probe <bounded|control> <png>"); process::exit(2) });
    let path = args.next().unwrap_or_else(|| { eprintln!("missing PNG path"); process::exit(2) });
    let bytes = fs::read(path).unwrap_or_else(|e| { eprintln!("read: {e}"); process::exit(3) });
    match mode.as_str() {
        "bounded" => match tuiscotti::sample::icc_expansion_probe(&bytes) {
            Ok((w, h, pixels)) => { assert_eq!((w, h, pixels.len()), (1, 1, 4)); println!("ok bounded {w}x{h} rgba={}", pixels.len()); }
            Err(e) => { eprintln!("bounded decode failed: {e:?}"); process::exit(4); }
        },
        "control" => {
            let decoder = image::codecs::png::PngDecoder::new(Cursor::new(bytes)).unwrap_or_else(|e| { eprintln!("control header failed: {e}"); process::exit(5) });
            let (w, h) = decoder.dimensions();
            let mut pixels = vec![0u8; decoder.total_bytes() as usize];
            decoder.read_image(&mut pixels).unwrap_or_else(|e| { eprintln!("control decode failed: {e}"); process::exit(6) });
            assert_eq!((w, h, pixels.len()), (1, 1, 4));
            println!("ok image::PngDecoder control {w}x{h} rgba={}", pixels.len());
        }
        _ => { eprintln!("unknown mode"); process::exit(2); }
    }
}
