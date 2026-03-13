use image::imageops::FilterType;
use image::GenericImageView;
use std::fs::File;
use std::io::{BufWriter, Cursor};
use std::path::Path;

const SIZES: &[u32] = &[16, 32, 48, 64, 128, 256];

fn main() {
    let input = Path::new("../../packaging/macos/icon.png");
    let output = Path::new("../../packaging/windows/rpview.ico");

    println!("Loading {}", input.display());
    let source = image::open(input).expect("Failed to open source PNG");
    let (w, h) = source.dimensions();
    println!("Source dimensions: {}x{}", w, h);

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for &size in SIZES {
        println!("  Resizing to {}x{}", size, size);
        let resized = source.resize_exact(size, size, FilterType::Lanczos3);
        let rgba = resized.to_rgba8();

        // Encode as PNG into memory
        let mut png_bytes = Vec::new();
        {
            let cursor = Cursor::new(&mut png_bytes);
            let mut encoder = png::Encoder::new(cursor, size, size);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("Failed to write PNG header");
            writer
                .write_image_data(rgba.as_raw())
                .expect("Failed to write PNG data");
        }

        let entry = ico::IconImage::read_png(Cursor::new(&png_bytes))
            .expect("Failed to read PNG into IconImage");
        icon_dir.add_entry(ico::IconDirEntry::encode(&entry).expect("Failed to encode ICO entry"));
    }

    let file = File::create(output).expect("Failed to create output ICO file");
    let writer = BufWriter::new(file);
    icon_dir.write(writer).expect("Failed to write ICO file");

    println!("Created {} with {} sizes", output.display(), SIZES.len());
}
