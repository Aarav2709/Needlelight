use std::{env, fs, io::Write, path::PathBuf};

const FALLBACK_ICON_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49,
    0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04,
    0x00, 0x00, 0x00, 0xB5, 0x1C, 0x0C, 0x02, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44,
    0x41, 0x54, 0x78, 0x9C, 0x63, 0x60, 0x60, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01,
    0x2B, 0x09, 0x4D, 0x84, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

fn ensure_fallback_icon() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let icons_dir = manifest_dir.join("icons");
    let icon_png = icons_dir.join("icon.png");
    let icon_512 = icons_dir.join("icon-512.png");
    let icon_icns = icons_dir.join("icon.icns");

    if let Some(parent) = icon_png.parent() {
        fs::create_dir_all(parent).expect("failed to create icons directory");
    }

    if !icon_png.exists() {
        let new_logo = manifest_dir
            .parent()
            .unwrap_or(&manifest_dir)
            .join("newlogo.png");
        if new_logo.exists() {
            fs::copy(&new_logo, &icon_png).expect("failed to copy newlogo.png to icons/icon.png");
        } else {
            fs::write(&icon_png, FALLBACK_ICON_PNG).expect("failed to write fallback icon");
        }
    }

    if !icon_512.exists() {
        let data = fs::read(&icon_png).unwrap_or_else(|_| FALLBACK_ICON_PNG.to_vec());
        fs::write(&icon_512, data).expect("failed to write icon-512.png");
    }

    if !icon_icns.exists() {
        let data = fs::read(&icon_512).unwrap_or_else(|_| FALLBACK_ICON_PNG.to_vec());
        write_icns(&icon_icns, &data).expect("failed to write icon.icns");
    }
}

fn write_icns(path: &PathBuf, png_bytes: &[u8]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    let block_len = (png_bytes.len() + 8) as u32;
    let file_len = block_len + 8;

    file.write_all(b"icns")?;
    file.write_all(&file_len.to_be_bytes())?;
    file.write_all(b"ic09")?;
    file.write_all(&block_len.to_be_bytes())?;
    file.write_all(png_bytes)?;
    Ok(())
}

fn main() {
    ensure_fallback_icon();
    tauri_build::build()
}
