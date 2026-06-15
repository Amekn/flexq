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

use clap::{ArgAction, Parser};
use qrcodegen::{QrCode, QrCodeEcc};
use std::error::Error;
use std::fs;
use std::io::{self, Read, Write};

/// Runtime configuration derived from command-line arguments.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The text or URL to encode into a QR code.
    #[arg(default_value = "")]
    text: String,

    /// The path where the SVG QR code will be saved.
    #[arg(default_value = "")]
    output: String,

    /// Read the text to encode from standard input.
    #[arg(short = 'i', long, action = ArgAction::SetTrue, conflicts_with = "source_file")]
    stdin: bool,

    /// Write the SVG QR code to standard output.
    #[arg(short = 'o', long, action = ArgAction::SetTrue)]
    stdout: bool,

    /// Read the text to encode from a file.
    #[arg(short = 's', long, conflicts_with = "stdin")]
    source_file: Option<String>,

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

    validate_color(&args.fg_color)?;
    validate_color(&args.bg_color)?;

    // Priority: stdin > source_file > text argument
    let text = if args.stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else if let Some(ref path) = args.source_file {
        fs::read_to_string(path)?
    } else {
        args.text
    };

    eprintln!("Generating QR code for {} bytes", text.len());

    let qr = QrCode::encode_text(&text, QrCodeEcc::Medium)?;
    let svg = qr_to_svg(&qr, args.border, &args.fg_color, &args.bg_color);

    if args.stdout {
        let mut stdout = io::stdout();
        stdout.write_all(svg.as_bytes())?;
        stdout.flush()?;
    } else if !args.output.is_empty() {
        fs::write(&args.output, svg)?;
        eprintln!("QR code saved to: {}", &args.output);
    }
    Ok(())
}

/// Validate that a color string is a valid hex color.
fn validate_color(color: &str) -> Result<(), Box<dyn Error>> {
    if !color.starts_with('#') || !color[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Invalid color: {}", color).into());
    }
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
fn qr_to_svg(qr: &QrCode, border: i32, fg_color: &str, bg_color: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_to_svg_contains_xml_declaration() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(&qr, 4, "#000000", "#FFFFFF");
        assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }

    #[test]
    fn qr_to_svg_contains_colors() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(&qr, 4, "#FF0000", "#FFFFCC");
        assert!(svg.contains("fill=\"#FFFFCC\""));
        assert!(svg.contains("fill=\"#FF0000\""));
    }

    #[test]
    fn qr_to_svg_border_affects_viewbox() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let size = qr.size() as i32;

        let svg_b0 = qr_to_svg(&qr, 0, "#000", "#FFF");
        assert!(svg_b0.contains(&format!(r#"viewBox="0 0 {} {}""#, size, size)));

        let svg_b8 = qr_to_svg(&qr, 8, "#000", "#FFF");
        let dim_b8 = size + 16;
        assert!(svg_b8.contains(&format!(r#"viewBox="0 0 {} {}""#, dim_b8, dim_b8)));
    }

    #[test]
    fn qr_to_svg_contains_crisp_edges() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(&qr, 4, "#000000", "#FFFFFF");
        assert!(svg.contains(r#"shape-rendering="crispEdges""#));
    }

    #[test]
    fn validate_color_rejects_invalid() {
        assert!(validate_color("red").is_err());
        assert!(validate_color("#GGG").is_err());
        assert!(validate_color("").is_err());
    }

    #[test]
    fn validate_color_accepts_valid() {
        assert!(validate_color("#000000").is_ok());
        assert!(validate_color("#FFF").is_ok());
        assert!(validate_color("#aB3cDe").is_ok());
    }
}
