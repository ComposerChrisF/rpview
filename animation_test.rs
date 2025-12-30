// Test program to understand image crate animation API
use image::codecs::gif::GifDecoder;
use image::codecs::webp::WebPDecoder;
use image::AnimationDecoder;
use std::fs::File;
use std::io::BufReader;

fn main() {
    println!("Testing GIF animation support...");
    
    // Test GIF
    if let Ok(file) = File::open("test.gif") {
        let reader = BufReader::new(file);
        if let Ok(decoder) = GifDecoder::new(reader) {
            if let Ok(frames) = decoder.into_frames().collect_frames() {
                println!("GIF: Found {} frames", frames.len());
                for (i, frame) in frames.iter().enumerate() {
                    let delay = frame.delay();
                    println!("  Frame {}: {}ms delay", i, delay.numer_denom_ms().0);
                }
            }
        }
    }
    
    println!("\nTesting WEBP animation support...");
    
    // Test WEBP
    if let Ok(file) = File::open("test.webp") {
        let reader = BufReader::new(file);
        if let Ok(decoder) = WebPDecoder::new(reader) {
            if decoder.has_animation() {
                println!("WEBP: Has animation");
                if let Ok(frames) = decoder.into_frames().collect_frames() {
                    println!("WEBP: Found {} frames", frames.len());
                    for (i, frame) in frames.iter().enumerate() {
                        let delay = frame.delay();
                        println!("  Frame {}: {}ms delay", i, delay.numer_denom_ms().0);
                    }
                }
            } else {
                println!("WEBP: Static image");
            }
        }
    }
}
