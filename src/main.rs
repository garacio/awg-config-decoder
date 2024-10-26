use std::env;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use flate2::read::ZlibDecoder;
use std::io::{self, Read};


fn main() {
    // Check if there's an argument provided
    let mut base64_input = if let Some(arg) = env::args().nth(1) {
        arg
    } else {
        // Check if there is data in stdin
        let mut input = String::new();
        if atty::is(atty::Stream::Stdin) {
            println!("Enter the Base64 string (may start with 'vpn://'):");
            io::stdin().read_line(&mut input).expect("Failed to read input");
        } else {
            io::stdin().read_to_string(&mut input).expect("Failed to read input from stdin");
        }
        input.trim().to_string()
    };

    // Remove BOM if present
    if base64_input.starts_with('\u{feff}') {
        base64_input = base64_input.trim_start_matches('\u{feff}').to_string();
    }

    // Check for "vpn://" prefix and remove it if present
    if base64_input.starts_with("vpn://") {
        base64_input = base64_input.replacen("vpn://", "", 1);
    }

    // Decode the input from Base64 (URL_SAFE_NO_PAD mode)
    let compressed_data = match URL_SAFE_NO_PAD.decode(&base64_input) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Base64 decoding error: {:?}", e);
            return;
        }
    };

    // Remove the Qt-specific header (first 4 bytes)
    let qt_compressed_data = &compressed_data[4..];

    // Decompress the data using Zlib
    let mut decoder = ZlibDecoder::new(qt_compressed_data);
    let mut decompressed_data = Vec::new();

    match decoder.read_to_end(&mut decompressed_data) {
        Ok(_) => {
            match String::from_utf8(decompressed_data) {
                Ok(text) => println!("{}", text),
                Err(e) => eprintln!("UTF-8 conversion error: {:?}", e),
            }
        }
        Err(e) => eprintln!("Decompression error: {:?}", e),
    }
}
