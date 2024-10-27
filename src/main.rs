use std::{env, process};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use flate2::read::ZlibDecoder;
use std::io::{self, Read, Write};
use clap;
use clap::{CommandFactory, Parser};
use flate2::Compression;
use flate2::write::ZlibEncoder;

#[derive(Parser)]
#[clap(name="awg-config-decoder", version)]
struct Args {
    /// Encode string
    #[clap(long, short='e')]
    encode: bool,
    /// Decode string
    #[clap(long, short='d')]
    decode: bool,
}

fn awg_config_decode(mut base64_input: String) {
    // Check for "vpn://" prefix and remove it if present
    if base64_input.starts_with("vpn://") {
        base64_input = base64_input.replacen("vpn://", "", 1);
    }

    // Decode the input from Base64 (URL_SAFE_NO_PAD mode)
    let compressed_data = match URL_SAFE_NO_PAD.decode(&base64_input) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Base64 decoding error: {:?} \n data {:?}", e, &base64_input);
            return;
        }
    };

    // let header = &compressed_data[0..4];
    // println!("Removed header bytes: {:?}", header);
    // let length_of_uncompressed_data = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    // println!("Expected uncompressed data length (from header): {}", length_of_uncompressed_data);

    // Remove the Qt-specific header (first 4 bytes)
    let qt_compressed_data = &compressed_data[4..];
    // println!("Compressed data length after removing header: {}", qt_compressed_data.len());
    // println!("Compressed data (hex): {:?}", qt_compressed_data);

    // Decompress the data using Zlib
    let mut decoder = ZlibDecoder::new(qt_compressed_data);
    let mut decompressed_data = Vec::new();

    match decoder.read_to_end(&mut decompressed_data) {
        Ok(_) => {
            // println!("Decompressed data length: {}", decompressed_data.len());
            match String::from_utf8(decompressed_data) {
                Ok(text) => println!("{}", text),
                Err(e) => eprintln!("UTF-8 conversion error: {:?}", e),
            }
        }
        Err(e) => eprintln!("Decompression error: {:?}", e),
    }
}

fn awg_config_encode(data: String) {
    let data_bytes = data.as_bytes();
    let mut qt_compressed_data = Vec::with_capacity(4 + data_bytes.len());
    qt_compressed_data.extend(&(data_bytes.len() as u32).to_be_bytes());

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(8));
    encoder.write_all(data_bytes).expect("Failed to compress data");
    let compressed_data = encoder.finish().expect("Failed to finish compression");

    qt_compressed_data.extend(compressed_data);
    let compressed_base64 = URL_SAFE_NO_PAD.encode(&qt_compressed_data);

    println!("vpn://{}", compressed_base64)
}

fn main() {
    let args = Args::parse();

    // Check if there's an argument provided
    let mut user_input = if let Some(arg) = env::args().nth(2) {
        arg
    } else {
        // Check if there is data in stdin
        let mut input = String::new();
        if atty::is(atty::Stream::Stdin) {
            println!("Enter string:");
            io::stdin().read_line(&mut input).expect("Failed to read input");
        } else {
            io::stdin().read_to_string(&mut input).expect("Failed to read input from stdin");
        }
        input.trim().to_string()
    };

    // Remove BOM if present
    if user_input.starts_with('\u{feff}') {
        user_input = user_input.trim_start_matches('\u{feff}').to_string();
    }

    // Проверка на наличие обоих флагов
    if args.encode && args.decode {
        eprintln!("Error: You can only select either encode (-e) or decode (-d), not both.");
        process::exit(1);
    }

    // Проверка, если ни один флаг не передан
    if !args.encode && !args.decode {
        // Выводим help и завершаем программу
        Args::command().print_help().unwrap();
        println!();
        process::exit(0);
    }

    // Запуск соответствующей функции
    if args.encode {
        awg_config_encode(user_input);
    } else if args.decode {
        awg_config_decode(user_input);
    }
}
