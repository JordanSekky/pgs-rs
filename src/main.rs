use pgs_rs::parse::parse_pgs;
use pgs_rs::render::{DisplaySetIterator, render_display_set};
use std::env;
use std::fs;
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file.sup>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    let mut data = match fs::read(filename) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    match parse_pgs(&mut data) {
        Ok(pgs) => {
            let temp_dir =
                tempdir::TempDir::new_in(".", "pgs_dump").expect("Failed to create temp dir");
            println!("Created temporary directory: {:?}", temp_dir.path());

            for (i, ds) in DisplaySetIterator::new(&pgs).enumerate() {
                if ds.is_empty() {
                    continue;
                }
                match render_display_set(&ds) {
                    Ok(rgba) => {
                        let file_path = temp_dir.path().join(format!("display_set_{}.png", i));
                        if let Err(e) = image::save_buffer(
                            &file_path,
                            &rgba,
                            ds.width as u32,
                            ds.height as u32,
                            image::ColorType::Rgba8,
                        ) {
                            eprintln!("Failed to save image to {:?}: {}", file_path, e);
                        } else {
                            println!("Saved {:?}", file_path);
                        }
                    }
                    Err(e) => eprintln!("Failed to render display set {}: {}", i, e),
                }
            }

            println!("Rendering complete. Press Ctrl+C to clean up and exit.");

            let running = Arc::new(AtomicBool::new(true));
            let r = running.clone();

            ctrlc::set_handler(move || {
                r.store(false, Ordering::SeqCst);
            })
            .expect("Error setting Ctrl-C handler");

            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(100));
            }

            println!("Removing temporary directory...");
        }
        Err(e) => {
            eprintln!("Failed to parse PGS data: {:?}", e.offset());
            // eprintln!("Failed to parse PGS data: {:?}", e);
            process::exit(1);
        }
    }
}
