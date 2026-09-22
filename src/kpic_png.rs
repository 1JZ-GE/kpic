//kpic-png
use image::{ImageError, ImageResult};
use imagequant::RGBA;
use std::io::Write;

/// quantize rgba pixels to a palette png.
pub fn quantize(rgba: &[u8], width: u32, height: u32, quality: u8) -> ImageResult<Vec<u8>> {
    let w = width as usize;
    let h = height as usize;
    if rgba.len() != w * h * 4 {
        return Err(img_err("internal: rgba buffer size mismatch"));
    }
    let mut attr = imagequant::new();
    attr.set_quality(quality.clamp(1, 100), 100).map_err(liq_err)?;
    attr.set_speed(1).map_err(liq_err)?;
    let pixels: Vec<RGBA> = rgba
        .chunks_exact(4)
        .map(|c| RGBA::new(c[0], c[1], c[2], c[3]))
        .collect();
    let mut img = attr.new_image(pixels, w, h, 0.0).map_err(liq_err)?;
    let mut result = attr.quantize(&mut img).map_err(liq_err)?;
    result.set_dithering_level(1.0).map_err(liq_err)?;
    let (palette, indexed) = result.remapped(&mut img).map_err(liq_err)?;
    Ok(encode_palette_png(&indexed, width, height, &palette))
}

fn liq_err(e: imagequant::Error) -> ImageError {
    img_err(&format!("libimagequant: {e}"))
}

fn img_err(msg: &str) -> ImageError {
    ImageError::Decoding(image::error::DecodingError::new(
        image::error::ImageFormatHint::Unknown,
        msg.to_string(),
    ))
}

/// write a bit-depth-8 palette png (colortype 3) from quantized indices.
fn encode_palette_png(indexed: &[u8], width: u32, height: u32, palette: &[RGBA]) -> Vec<u8> {
    let w = width as usize;
    let mut out = Vec::with_capacity(indexed.len() + w * 40);
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 3, 0, 0, 0]); // bitdepth 8, palette, no interlace
    chunk(&mut out, b"IHDR", &ihdr);

    let mut plte = Vec::with_capacity(palette.len() * 3);
    for c in palette {
        plte.extend_from_slice(&[c.r, c.g, c.b]);
    }
    chunk(&mut out, b"PLTE", &plte);

    // alpha byte for every entry up to the last translucent one
    let last_translucent = palette.iter().rposition(|c| c.a < 255);
    if let Some(last) = last_translucent {
        let trns: Vec<u8> = palette[..=last].iter().map(|c| c.a).collect();
        chunk(&mut out, b"tRNS", &trns);
    }

    let mut raw = Vec::with_capacity(indexed.len() + height as usize);
    for row in indexed.chunks(w) {
        raw.push(0); // filter none
        raw.extend_from_slice(row);
    }
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    enc.write_all(&raw).expect("zlib write");
    let idat = enc.finish().expect("zlib finish");
    chunk(&mut out, b"IDAT", &idat);

    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for b in kind.iter().chain(data) {
        crc ^= *b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (((crc & 1) as i32).wrapping_neg() as u32));
        }
    }
    out.extend_from_slice(&(crc ^ 0xFFFF_FFFF).to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ExtendedColorType, ImageEncoder};

    #[test]
    fn quantize_shrinks_and_roundtrips() {
        // 512x512 noisy-ish gradient, far more colors than a palette can hold
        let w = 512u32;
        let h = 512u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&[
                    (x % 256) as u8,
                    (y % 256) as u8,
                    ((x + y) % 256) as u8,
                    255,
                ]);
            }
        }
        let png = quantize(&rgba, w, h, 50).expect("quantize");

        // decoded palette png is valid and has matching dims
        let decoded =
            image::load_from_memory_with_format(&png, image::ImageFormat::Png).expect("decode");
        assert_eq!(decoded.width(), w);
        assert_eq!(decoded.height(), h);

        // source png (full-color re-encode) is the baseline to beat
        let mut buf = std::io::Cursor::new(Vec::new());
        image::codecs::png::PngEncoder::new(&mut buf)
            .write_image(&rgba, w, h, ExtendedColorType::Rgba8)
            .expect("encode baseline");
        assert!(png.len() < buf.get_ref().len(), "palette png larger than rgba png");
    }
}
