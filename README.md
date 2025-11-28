# pgs-rs

A Rust library for parsing and rendering PGS (Presentation Graphic Stream) subtitles, commonly found on Blu-ray discs.

## Features

-   **Parsing**: Efficiently parse PGS segments including Presentation Composition, Window Definition, Palette Definition, and Object Definition.
-   **Rendering**: Render display sets into raw RGBA buffers.
-   **Zero-copy parsing**: Uses `winnow` for fast, zero-copy parsing where possible.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
pgs-rs = "0.1.0"
```

## Usage

### Parsing and Rendering

Here is a basic example of how to load a `.sup` file, parse it, and iterate through the display sets.

```rust
use std::fs;
use pgs_rs::parse::parse_pgs;
use pgs_rs::render::{DisplaySetIterator, render_display_set};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load your PGS data (e.g., from a .sup file)
    let mut data = fs::read("subtitles.sup")?;

    // Parse the PGS stream
    let pgs = parse_pgs(&mut data).expect("Failed to parse PGS");

    // Iterate over each DisplaySet
    for (i, ds) in DisplaySetIterator::new(&pgs).enumerate() {
        if ds.is_empty() {
            continue;
        }
        
        println!("Rendering Display Set #{}", i);
        
        // Render the display set to an RGBA buffer
        match render_display_set(&ds) {
            Ok(rgba_buffer) => {
                println!("Rendered frame: {}x{}", ds.width, ds.height);
                // The buffer contains raw RGBA bytes: [r, g, b, a, r, g, b, a, ...]
                // You can save this to an image file using the `image` crate or process it further.
            }
            Err(e) => eprintln!("Error rendering: {}", e),
        }
    }

    Ok(())
}
```

## License

This project is licensed under the [MIT License](LICENSE).

