use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clap::{ArgGroup, Parser};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

#[derive(Parser)]
#[command(name = "awg-config-decoder", version)]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .args(["encode", "decode"]),
))]
struct Args {
    /// Encode string to a vpn:// URL
    #[arg(long, short = 'e')]
    encode: bool,

    /// Decode a vpn:// URL to the original config
    #[arg(long, short = 'd')]
    decode: bool,

    /// Input string (read from stdin if omitted)
    input: Option<String>,
}

fn awg_config_decode(input: &str) -> Result<String, String> {
    let payload = input.strip_prefix("vpn://").unwrap_or(input);

    let compressed = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| format!("Base64 decoding error: {e}"))?;

    // Qt prefixes zlib-compressed data with a 4-byte big-endian length header.
    if compressed.len() < 4 {
        return Err(format!(
            "Input too short: expected at least 4 bytes after Base64 decode, got {}",
            compressed.len()
        ));
    }

    let mut decoder = ZlibDecoder::new(&compressed[4..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Decompression error: {e}"))?;

    String::from_utf8(decompressed).map_err(|e| format!("UTF-8 conversion error: {e}"))
}

fn awg_config_encode(input: &str) -> Result<String, String> {
    let data = input.as_bytes();
    let mut output = Vec::with_capacity(4 + data.len());
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| format!("Compression error: {e}"))?;
    let compressed = encoder
        .finish()
        .map_err(|e| format!("Compression error: {e}"))?;
    output.extend_from_slice(&compressed);

    Ok(format!("vpn://{}", URL_SAFE_NO_PAD.encode(&output)))
}

fn read_input(explicit: Option<String>) -> io::Result<String> {
    let raw = if let Some(arg) = explicit {
        arg
    } else {
        let mut stdin = io::stdin();
        let mut buffer = String::new();
        if stdin.is_terminal() {
            println!("Enter string:");
            stdin.read_line(&mut buffer)?;
        } else {
            stdin.read_to_string(&mut buffer)?;
        }
        buffer.trim().to_string()
    };

    Ok(raw.trim_start_matches('\u{feff}').to_string())
}

fn main() -> ExitCode {
    let args = Args::parse();

    let input = match read_input(args.input) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("Failed to read input: {e}");
            return ExitCode::FAILURE;
        }
    };

    let result = if args.encode {
        awg_config_encode(&input)
    } else {
        awg_config_decode(&input)
    };

    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        let original = "[Interface]\nPrivateKey = abc\nAddress = 10.0.0.2/32\n";
        let encoded = awg_config_encode(original).expect("encode succeeds");
        assert!(encoded.starts_with("vpn://"));
        let decoded = awg_config_decode(&encoded).expect("decode succeeds");
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_accepts_payload_without_prefix() {
        let encoded = awg_config_encode("hello").expect("encode succeeds");
        let payload = encoded.strip_prefix("vpn://").unwrap();
        let decoded = awg_config_decode(payload).expect("decode succeeds");
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn decode_short_input_returns_error() {
        let err = awg_config_decode("AAA").unwrap_err();
        assert!(err.contains("too short"), "unexpected error: {err}");
    }

    #[test]
    fn decode_invalid_base64_returns_error() {
        let err = awg_config_decode("vpn://!!!not-base64!!!").unwrap_err();
        assert!(err.contains("Base64"), "unexpected error: {err}");
    }
}
