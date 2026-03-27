use image::RgbaImage;
use xnb::{MaybeCompressedXNB, SurfaceFormat, Texture2d, WindowSize};

pub fn load_texture_from_xnb(path: &str) -> Result<RgbaImage, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut reader = std::io::BufReader::new(file);
    let xnb = MaybeCompressedXNB::from_buffer(&mut reader)
        .map_err(|e| format!("XNB parse error: {:?}", e))?;

    let texture: Texture2d = match xnb {
        MaybeCompressedXNB::Uncompressed(xnb) => xnb
            .xnb()
            .map_err(|e| format!("Uncompressed error: {:?}", e))?
            .primary,
        MaybeCompressedXNB::Compressed(xnb) => xnb
            .xnb(WindowSize::KB64)
            .map_err(|e| format!("Compressed error: {:?}", e))?
            .primary,
    };

    // Check format
    if !matches!(texture.format, SurfaceFormat::Color) {
        return Err(format!(
            "Unsupported texture format: {:?} (expected RGBA)",
            texture.format
        ));
    }

    let data = &texture.mip_data[0];
    let width = texture.width as u32;
    let height = texture.height as u32;

    let img = RgbaImage::from_raw(width, height, data.clone())
        .ok_or("Failed to create RgbaImage from texture data")?;
    Ok(img)
}
