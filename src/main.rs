use pgs_rs::parse_pgs;
use std::env;
use std::fs;
use std::process;

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
        Ok(pgs) => println!("{:#?}", pgs),
        Err(e) => {
            eprintln!("Failed to parse PGS data: {:?}", e.offset());
            // eprintln!("Failed to parse PGS data: {:?}", e);
            process::exit(1);
        }
    }
}
