//! # FlexQ
//!
//! A minimal command-line QR code generator that encodes arbitrary text or URLs
//! into standalone SVG files. Built on top of the [`qrcodegen`] crate and
//! powered by [`clap`] for ergonomic argument parsing.
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
//! * `-h`, `--help` — Print help message and exit.
//! * `-V`, `--version` — Print version information and exit.
//!
//! # Details
//!
//! FlexQ generates SVG files using medium error correction (`QrCodeEcc::Medium`)
//! and a default border of 4 modules. The output SVG is standalone and can be
//! opened in any browser or vector graphics editor.

use clap::Parser;
use qrcodegen::{QrCode, QrCodeEcc};
// use std::env;
use std::error::Error;
use std::fs;
// use std::process;

/// The fixed border (in QR modules) around the code in the generated SVG.
const SVG_BORDER: i32 = 4;

/// Runtime configuration derived from command-line arguments.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The text or URL to encode into a QR code.
    text: String,
    /// The path where the SVG QR code will be saved.
    output: String,
}

/// The entry point for the FlexQ binary.
///
/// Parses arguments, generates a QR code, and writes it as an SVG file.
fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    println!("Generating QR code for: {}", &args.text);

    let qr = QrCode::encode_text(&args.text, QrCodeEcc::Medium)?;
    let svg = qr_to_svg(&qr, SVG_BORDER);
    fs::write(&args.output, svg)?;

    println!("QR code saved to: {}", &args.output);
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
