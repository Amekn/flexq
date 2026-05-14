//! # FlexQ
//!
//! A minimal command-line QR code generator that encodes arbitrary text into

//! an SVG file. Built on top of the [`qrcodegen`] crate.
//!
//! # Usage
//!
//! ```text
//! flexq "<text>" <output.svg>
//! ```
//!
//! # Examples
//!
//! ```text
//! flexq "https://example.com" qrcode.svg
//! flexq "Hello, world!" hello.svg
//! ```
//!
//! # Options
//!
//! * `-h`, `--help` — Print this help message and exit.
//!
//! # Details
//!
//! FlexQ generates SVG files using medium error correction (`QrCodeEcc::Medium`)
//! and a default border of 4 modules. The output SVG is standalone and can be
//! opened in any browser or vector graphics editor.

use qrcodegen::{QrCode, QrCodeEcc};
use std::env;
use std::error::Error;
use std::fs;
use std::process;

/// The fixed border (in QR modules) around the code in the generated SVG.
const SVG_BORDER: i32 = 4;

/// Help text shown when the user requests `-h` / `--help` or provides no arguments.
const HELP: &str = "\
FlexQ — a minimal QR code generator

Usage:
    flexq <text> <output.svg>

Arguments:
    <text>         The text or URL to encode into a QR code.
    <output.svg>   The path where the SVG QR code will be saved.

Options:
    -h, --help     Print this help message and exit.";

/// Runtime configuration derived from command-line arguments.
struct Config {
    /// The text or URL to encode into a QR code.
    input_text: String,
    /// The file path where the generated SVG will be written.
    output_path: String,
}

impl Config {
    /// Parse command-line arguments into a [`Config`].
    ///
    /// Returns `Err` with a description when arguments are missing or invalid.
    fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        // Skip the program name
        args.next();

        let first = args.next();

        // Handle explicit help flags or completely missing arguments
        if first.is_none() || first.as_ref().is_some_and(|s| s == "-h" || s == "--help") {
            return Err("");
        }

        let input_text = first.unwrap();
        let output_path = args.next().ok_or("Missing output file path.")?;

        Ok(Config {
            input_text,
            output_path,
        })
    }
}

/// The entry point for the FlexQ binary.
///
/// Parses arguments, generates a QR code, and writes it as an SVG file.
fn main() -> Result<(), Box<dyn Error>> {
    let config = match Config::build(env::args()) {
        Ok(c) => c,
        Err(msg) => {
            if msg.is_empty() {
                // No error detail — just print help
                println!("{HELP}");
                process::exit(0);
            } else {
                eprintln!("Problem parsing arguments: {msg}");
                eprintln!();
                eprintln!("{HELP}");
                process::exit(1);
            }
        }
    };

    println!("Generating QR code for: {}", config.input_text);

    let qr = QrCode::encode_text(&config.input_text, QrCodeEcc::Medium)?;
    let svg = qr_to_svg(&qr, SVG_BORDER);
    fs::write(&config.output_path, svg)?;

    println!("QR code saved to: {}", config.output_path);
    Ok(())
}

/// Convert a [`QrCode`] into an SVG string.
///
/// # Arguments
///
/// * `qr` — The generated QR code to render.
/// * `border` — Number of empty modules to leave around the code.
///
/// # Returns
///
/// A valid, self-contained SVG document as a [`String`].
fn qr_to_svg(qr: &QrCode, border: i32) -> String {
    let size = qr.size();
    let dimension = size + 2 * border;
    let mut paths = String::new();

    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                paths.push_str(&format!("M{},{}h1v1h-1z ", x + border, y + border));
            }
        }
    }

    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
 viewBox="0 0 {dimension} {dimension}"
 shape-rendering="crispEdges">
<rect width="100%" height="100%" fill="#FFFFFF"/>
<path d="{paths}" fill="#000000"/>
</svg>"##
    )
}
