//! # FlexQ
//!
//! A minimal command-line QR code generator that encodes arbitrary text or URLs
//! into standalone SVG files. Built on top of the [`qrcodegen`] crate and
//! powered by [`clap`] for ergonomic argument parsing.
//!
//! # Usage
//!
//! ```text
//! flexq "<text>" <output.svg> [OPTIONS]
//! ```
//!
//! # Examples
//!
//! ```text
//! flexq "https://example.com" qrcode.svg
//! flexq "Hello, world!" hello.svg
//! flexq "https://example.com" qrcode.svg --border 8
//! flexq "https://example.com" qrcode.svg --fg-color "#FF0000" --bg-color "#FFFFCC"
//! ```
//!
//! # Options
//!
//! * `-b`, `--border` — Border size in QR modules (default: `4`).
//! * `-F`, `--fg-color` — Foreground color of the QR code (default: `#000000`).
//! * `-B`, `--bg-color` — Background color of the QR code (default: `#FFFFFF`).
//! * `-h`, `--help` — Print help message and exit.
//! * `-V`, `--version` — Print version information and exit.
//!
//! # Details
//!
//! FlexQ generates SVG files using medium error correction (`QrCodeEcc::Medium`)
//! and a configurable border (default: 4 modules), foreground color (default: black),
//! and background color (default: white). The output SVG is standalone and can be
//! opened in any browser or vector graphics editor.

use clap::Parser;
use qrcodegen::{QrCode, QrCodeEcc};
// use std::env;
use std::error::Error;
use std::fs;
// use std::process;

/// Runtime configuration derived from command-line arguments.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The text or URL to encode into a QR code.
    text: String,
    /// The path where the SVG QR code will be saved.
    output: String,
    /// The border size (in QR modules) around the QR code in the generated SVG.
    #[arg(short = 'b', long, default_value = "4")]
    border: i32,
    /// The foreground color of the QR code in the generated SVG.
    #[arg(short = 'F', long, default_value = "#000000")]
    fg_color: String,
    /// The background color of the QR code in the generated SVG.
    #[arg(short = 'B', long, default_value = "#FFFFFF")]
    bg_color: String,
}

/// The entry point for the FlexQ binary.
///
/// Parses arguments, generates a QR code, and writes it as an SVG file.
fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    println!("Generating QR code for: {}", &args.text);

    let qr = QrCode::encode_text(&args.text, QrCodeEcc::Medium)?;
    let svg = qr_to_svg(&qr, &args.border, &args.fg_color, &args.bg_color);
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
fn qr_to_svg(qr: &QrCode, border: &i32, fg_color: &str, bg_color: &str) -> String {
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
<rect width="100%" height="100%" fill="{bg_color}"/>
<path d="{paths}" fill="{fg_color}"/>
</svg>"##
    )
}
