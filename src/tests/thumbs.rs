use super::*;

#[test]
fn truncated_cache_entries_are_rejected() {
    assert!(!looks_decodable(b""));
    assert!(!looks_decodable(b"\xFF\xD8\xFF"), "a 3-byte jpeg header is not an image");
    assert!(!looks_decodable(b"<!DOCTYPE html><html>"));
}

#[test]
fn real_headers_are_accepted() {
    let jpeg = [0xFFu8, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0, 0, 0, 0, 0];
    let png = [0x89u8, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 0];
    let mut webp = Vec::from(*b"RIFF____WEBP");
    webp.extend_from_slice(&[0u8; 8]);

    assert!(looks_decodable(&jpeg));
    assert!(looks_decodable(&png));
    assert!(looks_decodable(&webp));
}

#[test]
fn oversized_screenshots_are_shrunk_and_small_ones_left_alone() {
    use image::{ImageBuffer, Rgb};

    let encode = |w: u32, h: u32| {
        let buffer = ImageBuffer::from_fn(w, h, |x, y| Rgb([(x % 255) as u8, (y % 255) as u8, 128]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(buffer).write_to(&mut out, image::ImageFormat::Jpeg).unwrap();
        out.into_inner()
    };

    let big = encode(1920, 1080);
    let small = shrink(&big).expect("a 1920 wide image should shrink");
    assert!(looks_decodable(&small));
    assert!(small.len() < big.len(), "shrunk {} vs original {}", small.len(), big.len());

    let already_small = encode(400, 225);
    assert!(shrink(&already_small).is_none(), "no point re-encoding it");
}

#[test]
fn decoding_yields_pixels_at_the_capped_size() {
    use image::{ImageBuffer, Rgb};

    let buffer = ImageBuffer::from_fn(1920, 1080, |x, _| Rgb([(x % 255) as u8, 40, 90]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(buffer).write_to(&mut encoded, image::ImageFormat::Jpeg).unwrap();

    let (w, h, pixels) = decode_rgba(&encoded.into_inner()).expect("decode");
    assert_eq!(w, 640, "oversized art is capped before it reaches memory");
    assert_eq!(pixels.len(), (w * h * 4) as usize);
}

#[test]
fn cache_names_are_stable_and_distinct() {
    let dir = PathBuf::from("/tmp");
    let a = cached_path(&dir, "https://example.com/one.jpg");
    let b = cached_path(&dir, "https://example.com/two.jpg");

    assert_ne!(a, b);
    assert_eq!(a, cached_path(&dir, "https://example.com/one.jpg"));
    assert!(!a.file_name().unwrap().to_string_lossy().contains(char::is_whitespace));
}
