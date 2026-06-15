//! # FlexQ
//!
//! A flexible command-line QR code generator that encodes arbitrary text, URLs,
//! WiFi credentials, or vCard contacts into standalone SVG or PNG files.
//!
//! Built on top of the [`qrcodegen`] crate and powered by [`clap`] for ergonomic
//! argument parsing.
//!
//! # Usage
//!
//! ```text
//! flexq [TEXT] [OPTIONS]
//! ```
//!
//! # Examples
//!
//! ```text
//! flexq "https://example.com" -o qrcode.svg
//! flexq "Hello, world!" -o hello.svg --shape rounded
//! flexq "https://example.com" --format png --size 400 -o qrcode.png
//! flexq --wifi --ssid "Home" --password "secret" --wifitype wpa -o wifi.svg
//! flexq --vcard --name "John" --phone "123" --email "john@example.com" -o contact.svg
//! echo "https://example.com" | flexq --stdin --stdout
//! flexq --batch codes.csv
//! flexq "https://example.com" --term
//! ```

use clap::{ArgAction, Parser, ValueEnum};
use qrcodegen::{Mask, QrCode, QrCodeEcc, QrSegment, Version};
use std::error::Error;
use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::Path;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

/// A flexible command-line QR code generator.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The text or URL to encode into a QR code.
    #[arg(default_value = "")]
    text: String,

    /// The file path where the QR code will be saved.
    #[arg(short = 'o', long)]
    output: Option<String>,

    /// Read the text to encode from standard input.
    #[arg(short = 'i', long, action = ArgAction::SetTrue, conflicts_with = "source_file")]
    stdin: bool,

    /// Write the QR code to standard output.
    #[arg(long, action = ArgAction::SetTrue)]
    stdout: bool,

    /// Read the text to encode from a file.
    #[arg(short = 's', long, conflicts_with = "stdin")]
    source_file: Option<String>,

    // --- WiFi mode ---
    /// Generate a WiFi configuration QR code.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["vcard", "batch"])]
    wifi: bool,

    /// WiFi network name (SSID).
    #[arg(long, requires = "wifi")]
    ssid: Option<String>,

    /// WiFi password.
    #[arg(long, requires = "wifi")]
    password: Option<String>,

    /// WiFi encryption type (default: wpa).
    #[arg(long, requires = "wifi", value_enum, default_value = "wpa")]
    wifitype: WifiType,

    // --- vCard mode ---
    /// Generate a vCard contact QR code.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["wifi", "batch"])]
    vcard: bool,

    /// Contact full name.
    #[arg(long, requires = "vcard")]
    name: Option<String>,

    /// Contact phone number.
    #[arg(long, requires = "vcard")]
    phone: Option<String>,

    /// Contact email address.
    #[arg(long, requires = "vcard")]
    email: Option<String>,

    /// Contact organization.
    #[arg(long, requires = "vcard")]
    org: Option<String>,

    /// Contact job title.
    #[arg(long, requires = "vcard")]
    title: Option<String>,

    /// Contact website URL.
    #[arg(long, requires = "vcard")]
    url: Option<String>,

    // --- Output format ---
    /// Output format (default: svg).
    #[arg(long, value_enum, default_value = "svg")]
    format: OutputFormat,

    /// Print QR code to terminal using Unicode block characters.
    #[arg(long, action = ArgAction::SetTrue)]
    term: bool,

    /// Fixed output size in pixels (default: module-based).
    #[arg(long, value_parser = parse_positive_i32)]
    size: Option<i32>,

    // --- Appearance ---
    /// Module shape (default: square).
    #[arg(long, value_enum, default_value = "square")]
    shape: ModuleShape,

    /// Foreground (module) color in hex (default: #000000).
    #[arg(short = 'F', long, default_value = "#000000")]
    fg_color: String,

    /// Background color in hex (default: #FFFFFF).
    #[arg(short = 'B', long, default_value = "#FFFFFF")]
    bg_color: String,

    /// Invert default colors for dark mode (fg=#FFFFFF, bg=#1A1A1A).
    #[arg(long, action = ArgAction::SetTrue)]
    dark_mode: bool,

    /// Border size in QR modules (default: 4).
    #[arg(short = 'b', long, default_value = "4", value_parser = parse_non_negative_i32)]
    border: i32,

    /// Error correction level (default: M).
    #[arg(long, default_value = "M")]
    ecc: String,

    /// Mask pattern 0–7 (default: auto).
    #[arg(long, value_parser = parse_mask)]
    mask: Option<u8>,

    /// Overlay a logo image in the center of the QR code.
    #[arg(long)]
    logo: Option<String>,

    // --- Batch mode ---
    /// Path to a CSV/TSV file with `text,output_path` rows for batch generation.
    #[arg(long, conflicts_with_all = ["wifi", "vcard", "stdin", "source_file"])]
    batch: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
#[clap(rename_all = "kebab-case")]
enum ModuleShape {
    Square,
    Rounded,
    Circle,
    Hexagon,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
#[clap(rename_all = "kebab-case")]
enum WifiType {
    Wpa,
    Wep,
    None,
}

impl WifiType {
    fn qr_type(self) -> &'static str {
        match self {
            Self::Wpa => "WPA",
            Self::Wep => "WEP",
            Self::None => "nopass",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
#[clap(rename_all = "kebab-case")]
enum OutputFormat {
    Svg,
    Png,
}

fn parse_positive_i32(value: &str) -> Result<i32, String> {
    let value = value
        .parse::<i32>()
        .map_err(|_| format!("'{}' must be a whole number", value))?;

    if value > 0 {
        Ok(value)
    } else {
        Err("value must be greater than 0".to_string())
    }
}

fn parse_non_negative_i32(value: &str) -> Result<i32, String> {
    let value = value
        .parse::<i32>()
        .map_err(|_| format!("'{}' must be a whole number", value))?;

    if value >= 0 {
        Ok(value)
    } else {
        Err("value must be 0 or greater".to_string())
    }
}

fn parse_mask(value: &str) -> Result<u8, String> {
    let value = value
        .parse::<u8>()
        .map_err(|_| format!("'{}' must be a whole number from 0 to 7", value))?;

    if value <= 7 {
        Ok(value)
    } else {
        Err("mask must be between 0 and 7".to_string())
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Apply dark mode defaults
    let (fg_color, bg_color) = if args.dark_mode {
        ("#FFFFFF".to_string(), "#1A1A1A".to_string())
    } else {
        (args.fg_color.clone(), args.bg_color.clone())
    };

    validate_color(&fg_color)?;
    validate_color(&bg_color)?;

    let ecc = parse_ecc(&args.ecc)?;

    if let Some(ref batch_path) = args.batch {
        return run_batch(batch_path, &args, &fg_color, &bg_color, ecc);
    }

    // Build the text to encode
    let text = if args.wifi {
        build_wifi_uri(&args)?
    } else if args.vcard {
        build_vcard(&args)?
    } else if args.stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else if let Some(ref path) = args.source_file {
        fs::read_to_string(path)?
    } else {
        args.text.clone()
    };

    if text.is_empty() {
        return Err("No input text provided. Use TEXT, --stdin, --source-file, --wifi, --vcard, or --batch.".into());
    }

    eprintln!("Generating QR code for {} bytes", text.len());

    let qr = encode_qr(&text, ecc, args.mask)?;

    // Terminal preview
    if args.term {
        render_terminal(&qr);
        if args.output.is_none() && !args.stdout {
            return Ok(());
        }
    }

    // Output
    let output_path = args.output.clone();

    if args.stdout || output_path.is_some() {
        match args.format {
            OutputFormat::Png => {
                let png = qr_to_png(&qr, args.border, &fg_color, &bg_color, args.size)?;
                if args.stdout {
                    io::stdout().write_all(&png)?;
                } else if let Some(ref path) = output_path {
                    fs::write(path, &png)?;
                    eprintln!("QR code saved to: {}", path);
                }
            }
            OutputFormat::Svg => {
                let svg = qr_to_svg(
                    &qr,
                    args.border,
                    &fg_color,
                    &bg_color,
                    &args.shape,
                    args.size,
                    &args.logo,
                );
                if args.stdout {
                    io::stdout().write_all(svg.as_bytes())?;
                } else if let Some(ref path) = output_path {
                    fs::write(path, &svg)?;
                    eprintln!("QR code saved to: {}", path);
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Batch mode
// ---------------------------------------------------------------------------

fn run_batch(
    path: &str,
    args: &Args,
    fg_color: &str,
    bg_color: &str,
    ecc: QrCodeEcc,
) -> Result<(), Box<dyn Error>> {
    let file = fs::File::open(path)?;
    let delimiter = if path.ends_with(".tsv") { b'\t' } else { b',' };

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .from_reader(file);

    let mut success = 0;
    let mut failed = 0;

    for (i, result) in reader.records().enumerate() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Row {}: parse error: {}", i + 1, e);
                failed += 1;
                continue;
            }
        };

        if record.len() < 2 {
            eprintln!(
                "Row {}: expected at least 2 columns (text, output_path)",
                i + 1
            );
            failed += 1;
            continue;
        }

        let text = record[0].trim();
        let output = record[1].trim();

        if text.is_empty() {
            eprintln!("Row {}: empty text, skipping", i + 1);
            failed += 1;
            continue;
        }

        match encode_qr(text, ecc, args.mask) {
            Ok(qr) => {
                if args.term {
                    render_terminal(&qr);
                }

                match args.format {
                    OutputFormat::Png => {
                        let png = qr_to_png(&qr, args.border, fg_color, bg_color, args.size)?;
                        fs::write(output, &png)?;
                    }
                    OutputFormat::Svg => {
                        let svg = qr_to_svg(
                            &qr,
                            args.border,
                            fg_color,
                            bg_color,
                            &args.shape,
                            args.size,
                            &args.logo,
                        );
                        fs::write(output, &svg)?;
                    }
                }
                eprintln!("Row {}: saved to {}", i + 1, output);
                success += 1;
            }
            Err(e) => {
                eprintln!("Row {}: encode error: {}", i + 1, e);
                failed += 1;
            }
        }
    }

    eprintln!("Batch complete: {} succeeded, {} failed", success, failed);
    Ok(())
}

fn encode_qr(text: &str, ecc: QrCodeEcc, mask: Option<u8>) -> Result<QrCode, Box<dyn Error>> {
    let mask = mask.map(Mask::new);
    let segments = QrSegment::make_segments(text);

    Ok(QrCode::encode_segments_advanced(
        &segments,
        ecc,
        Version::MIN,
        Version::MAX,
        mask,
        true,
    )?)
}

// ---------------------------------------------------------------------------
// WiFi URI builder
// ---------------------------------------------------------------------------

fn build_wifi_uri(args: &Args) -> Result<String, Box<dyn Error>> {
    let ssid = args
        .ssid
        .as_deref()
        .ok_or("--ssid is required with --wifi")?;
    let password = args.password.as_deref().unwrap_or("");
    let r#type = args.wifitype.qr_type();

    Ok(format!(
        "WIFI:T:{};S:{};P:{};;",
        r#type,
        escape_wifi_field(ssid),
        escape_wifi_field(password)
    ))
}

fn escape_wifi_field(value: &str) -> String {
    let mut escaped = String::new();

    for ch in value.chars() {
        if matches!(ch, '\\' | ';' | ',' | ':' | '"') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }

    escaped
}

// ---------------------------------------------------------------------------
// vCard builder
// ---------------------------------------------------------------------------

fn build_vcard(args: &Args) -> Result<String, Box<dyn Error>> {
    let name = args.name.as_deref().unwrap_or("");
    let phone = args.phone.as_deref().unwrap_or("");
    let email = args.email.as_deref().unwrap_or("");
    let org = args.org.as_deref().unwrap_or("");
    let title = args.title.as_deref().unwrap_or("");
    let url = args.url.as_deref().unwrap_or("");

    let mut vcard = String::from("BEGIN:VCARD\nVERSION:3.0\n");
    if !name.is_empty() {
        vcard.push_str(&format!("FN:{}\n", name));
        vcard.push_str(&format!("N:{};;\n", name));
    }
    if !phone.is_empty() {
        vcard.push_str(&format!("TEL:{}\n", phone));
    }
    if !email.is_empty() {
        vcard.push_str(&format!("EMAIL:{}\n", email));
    }
    if !org.is_empty() {
        vcard.push_str(&format!("ORG:{}\n", org));
    }
    if !title.is_empty() {
        vcard.push_str(&format!("TITLE:{}\n", title));
    }
    if !url.is_empty() {
        vcard.push_str(&format!("URL:{}\n", url));
    }
    vcard.push_str("END:VCARD");

    Ok(vcard)
}

// ---------------------------------------------------------------------------
// Terminal renderer
// ---------------------------------------------------------------------------

fn render_terminal(qr: &QrCode) {
    let size = qr.size();
    // Header
    println!("\n  ┌──────────────────────────────┐");
    println!("  │         FlexQ Preview         │");
    println!("  └──────────────────────────────┘\n");

    for y in 0..size {
        let mut line = String::new();
        for x in 0..size {
            if qr.get_module(x, y) {
                line.push_str("██");
            } else {
                line.push_str("  ");
            }
        }
        println!("  {}  ", line);
    }
    println!();
}

// ---------------------------------------------------------------------------
// SVG renderer
// ---------------------------------------------------------------------------

fn qr_to_svg(
    qr: &QrCode,
    border: i32,
    fg_color: &str,
    bg_color: &str,
    shape: &ModuleShape,
    fixed_size: Option<i32>,
    logo_path: &Option<String>,
) -> String {
    let module_size = qr.size();
    let dimension = module_size + 2 * border;

    // Use fixed size if provided, otherwise module-based
    let svg_size = fixed_size.unwrap_or(dimension);
    let scale = svg_size as f64 / dimension as f64;

    let version_comment = format!("<!-- generated by flexq {} -->", env!("CARGO_PKG_VERSION"));

    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
{comment}
<svg xmlns="http://www.w3.org/2000/svg"
 viewBox="0 0 {svg_size} {svg_size}"
 width="{svg_size}" height="{svg_size}"
 shape-rendering="crispEdges">
<rect width="100%" height="100%" fill="{bg_color}"/>
"#,
        comment = version_comment,
        svg_size = svg_size,
        bg_color = bg_color
    );

    // Build modules based on shape
    match shape {
        ModuleShape::Square => {
            svg.push_str(&build_square_path(qr, border, scale, fg_color));
        }
        ModuleShape::Rounded => {
            svg.push_str(&build_rounded_path(qr, border, scale, fg_color));
        }
        ModuleShape::Circle => {
            svg.push_str(&build_circle_path(qr, border, scale, fg_color));
        }
        ModuleShape::Hexagon => {
            svg.push_str(&build_hexagon_path(qr, border, scale, fg_color));
        }
    }

    // Logo overlay
    if let Some(logo) = logo_path {
        svg.push_str(&build_logo_overlay(qr, border, scale, logo, bg_color));
    }

    svg.push_str("</svg>\n");
    svg
}

/// Square modules (default) — efficient single path.
fn build_square_path(qr: &QrCode, border: i32, scale: f64, fg_color: &str) -> String {
    let size = qr.size();
    let mut d = String::new();

    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                let px = (x + border) as f64 * scale;
                let py = (y + border) as f64 * scale;
                let s = scale;
                d.push_str(&format!("M{},{}h{}v{}h-{}z ", px, py, s, s, s));
            }
        }
    }

    format!("<path d=\"{}\" fill=\"{}\"/>\n", d, fg_color)
}

/// Rounded rectangle modules.
fn build_rounded_path(qr: &QrCode, border: i32, scale: f64, fg_color: &str) -> String {
    let size = qr.size();
    let r = (scale * 0.3).min(scale * 0.45);
    let mut rects = String::new();

    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                let px = (x + border) as f64 * scale;
                let py = (y + border) as f64 * scale;
                let s = scale;
                rects.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\"/> ",
                    px, py, s, s, r, r
                ));
            }
        }
    }

    format!("<g fill=\"{}\">{}</g>\n", fg_color, rects)
}

/// Circle modules.
fn build_circle_path(qr: &QrCode, border: i32, scale: f64, fg_color: &str) -> String {
    let size = qr.size();
    let r = scale * 0.48;
    let mut circles = String::new();

    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                let cx = (x + border) as f64 * scale + scale * 0.5;
                let cy = (y + border) as f64 * scale + scale * 0.5;
                circles.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/> ",
                    cx, cy, r
                ));
            }
        }
    }

    format!("<g fill=\"{}\">{}</g>\n", fg_color, circles)
}

/// Hexagon modules.
fn build_hexagon_path(qr: &QrCode, border: i32, scale: f64, fg_color: &str) -> String {
    let size = qr.size();
    let r = scale * 0.48;
    let mut polys = String::new();

    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                let cx = (x + border) as f64 * scale + scale * 0.5;
                let cy = (y + border) as f64 * scale + scale * 0.5;
                // Pointy-top hexagon
                let pts = [
                    (cx, cy - r),
                    (cx + r * 0.866, cy - r * 0.5),
                    (cx + r * 0.866, cy + r * 0.5),
                    (cx, cy + r),
                    (cx - r * 0.866, cy + r * 0.5),
                    (cx - r * 0.866, cy - r * 0.5),
                ];
                let point_str: Vec<String> = pts
                    .iter()
                    .map(|(px, py)| format!("{},{}", px, py))
                    .collect();
                polys.push_str(&format!("<polygon points=\"{}\"/> ", point_str.join(" ")));
            }
        }
    }

    format!("<g fill=\"{}\">{}</g>\n", fg_color, polys)
}

/// Logo overlay — carves out a center region and embeds the logo as an image.
fn build_logo_overlay(
    qr: &QrCode,
    border: i32,
    scale: f64,
    logo_path: &str,
    bg_color: &str,
) -> String {
    let module_size = qr.size();

    // Logo region: center 25% of the module area (not including border)
    let logo_module_fraction = 0.25;
    let logo_modules = (module_size as f64 * logo_module_fraction).ceil() as i32;
    let logo_border_offset = (module_size - logo_modules) / 2;

    let logo_x = (logo_border_offset + border) as f64 * scale;
    let logo_y = (logo_border_offset + border) as f64 * scale;
    let logo_w = logo_modules as f64 * scale;
    let logo_h = logo_w;

    // Quiet zone padding around the logo
    let quiet = scale * 2.0;

    // Encode the logo file path as a data URI for embedding
    let logo_data = match fs::read(logo_path) {
        Ok(data) => {
            let ext = Path::new(logo_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            let mime = match ext.to_lowercase().as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                _ => "image/png",
            };
            let b64 = base64_encode(&data);
            format!("data:{};base64,{}", mime, b64)
        }
        Err(e) => {
            eprintln!("Warning: could not read logo '{}': {}", logo_path, e);
            return String::new();
        }
    };

    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" rx=\"{}\"/>\n<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"{}\"/>\n",
        logo_x - quiet,
        logo_y - quiet,
        logo_w + quiet * 2.0,
        logo_h + quiet * 2.0,
        bg_color,
        scale,
        logo_x,
        logo_y,
        logo_w,
        logo_h,
        logo_data
    )
}

/// Minimal base64 encoder (avoids adding a dependency just for logo embedding).
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let chunks = data.chunks(3);

    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

// ---------------------------------------------------------------------------
// PNG renderer
// ---------------------------------------------------------------------------

fn qr_to_png(
    qr: &QrCode,
    border: i32,
    fg_color: &str,
    bg_color: &str,
    fixed_size: Option<i32>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let module_size = qr.size();
    let dimension = module_size + 2 * border;
    let png_size = fixed_size.unwrap_or(dimension);
    let actual_size = u32::try_from(png_size)?;

    let mut img = image::ImageBuffer::new(actual_size, actual_size);
    let bg = parse_rgb(bg_color)?;
    let fg = parse_rgb(fg_color)?;

    // Fill background
    for pixel in img.pixels_mut() {
        *pixel = image::Rgb(bg);
    }

    // Map output pixels back to QR modules so --size is the exact PNG dimension.
    for y in 0..actual_size {
        let module_y =
            ((i64::from(y) * i64::from(dimension)) / i64::from(actual_size)) as i32 - border;
        for x in 0..actual_size {
            let module_x =
                ((i64::from(x) * i64::from(dimension)) / i64::from(actual_size)) as i32 - border;

            if qr.get_module(module_x, module_y) {
                img.put_pixel(x, y, image::Rgb(fg));
            }
        }
    }

    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)?;
    Ok(buf.into_inner())
}

/// Parse a hex color string into an [R, G, B] array.
fn parse_rgb(color: &str) -> Result<[u8; 3], Box<dyn Error>> {
    let hex = color.strip_prefix('#').ok_or("Color must start with '#'")?;
    let rgb: [u8; 3] = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16)?;
            let g = u8::from_str_radix(&hex[1..2], 16)?;
            let b = u8::from_str_radix(&hex[2..3], 16)?;
            [r * 17, g * 17, b * 17]
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            [r, g, b]
        }
        _ => return Err(format!("Invalid hex color length: {}", color).into()),
    };
    Ok(rgb)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate that a color string is a valid hex color.
fn validate_color(color: &str) -> Result<(), Box<dyn Error>> {
    let hex = color.strip_prefix('#').ok_or(format!(
        "Invalid color '{}': must start with '#' (e.g. #FF0000 or #F00)",
        color
    ))?;

    if hex.len() != 3 && hex.len() != 6 {
        return Err(format!(
            "Invalid color '{}': must be 3 or 6 hex digits after '#'",
            color
        )
        .into());
    }

    for (i, c) in hex.chars().enumerate() {
        if !c.is_ascii_hexdigit() {
            return Err(format!(
                "Invalid color '{}': '{}' at position {} is not a valid hex digit",
                color,
                c,
                i + 1
            )
            .into());
        }
    }

    Ok(())
}

/// Parse an error correction level string.
fn parse_ecc(level: &str) -> Result<QrCodeEcc, Box<dyn Error>> {
    match level {
        "L" => Ok(QrCodeEcc::Low),
        "M" => Ok(QrCodeEcc::Medium),
        "Q" => Ok(QrCodeEcc::Quartile),
        "H" => Ok(QrCodeEcc::High),
        _ => Err(format!("Invalid ECC level '{}': must be L, M, Q, or H", level).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Color validation ---

    #[test]
    fn validate_color_rejects_no_hash() {
        assert!(validate_color("red").is_err());
        assert!(validate_color("FF0000").is_err());
    }

    #[test]
    fn validate_color_rejects_bad_hex() {
        assert!(validate_color("#GGG").is_err());
        assert!(validate_color("#ZZZZZZ").is_err());
    }

    #[test]
    fn validate_color_rejects_empty() {
        assert!(validate_color("").is_err());
        assert!(validate_color("#").is_err());
    }

    #[test]
    fn validate_color_accepts_3_digit() {
        assert!(validate_color("#000").is_ok());
        assert!(validate_color("#FFF").is_ok());
        assert!(validate_color("#aB3").is_ok());
    }

    #[test]
    fn validate_color_accepts_6_digit() {
        assert!(validate_color("#000000").is_ok());
        assert!(validate_color("#FFFFFF").is_ok());
        assert!(validate_color("#aB3cDe").is_ok());
    }

    #[test]
    fn validate_color_error_message_is_helpful() {
        let err = validate_color("#GGG").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("#"), "error should mention the color");
        assert!(msg.contains("hex"), "error should mention hex");
    }

    // --- ECC parsing ---

    #[test]
    fn parse_ecc_all_levels() {
        assert!(parse_ecc("L").is_ok());
        assert!(parse_ecc("M").is_ok());
        assert!(parse_ecc("Q").is_ok());
        assert!(parse_ecc("H").is_ok());
    }

    #[test]
    fn parse_ecc_rejects_invalid() {
        assert!(parse_ecc("X").is_err());
        assert!(parse_ecc("low").is_err());
    }

    // --- CLI validation ---

    #[test]
    fn cli_rejects_invalid_output_format() {
        assert!(Args::try_parse_from(["flexq", "text", "--format", "pdf"]).is_err());
    }

    #[test]
    fn cli_rejects_invalid_wifi_type() {
        assert!(
            Args::try_parse_from(["flexq", "--wifi", "--ssid", "Cafe", "--wifitype", "bogus"])
                .is_err()
        );
    }

    #[test]
    fn cli_rejects_invalid_numeric_options() {
        assert!(Args::try_parse_from(["flexq", "text", "--border=-1"]).is_err());
        assert!(Args::try_parse_from(["flexq", "text", "--size=0"]).is_err());
        assert!(Args::try_parse_from(["flexq", "text", "--mask=8"]).is_err());
    }

    #[test]
    fn encode_qr_uses_requested_mask() {
        let qr = encode_qr("test", QrCodeEcc::Medium, Some(3)).unwrap();
        assert_eq!(qr.mask().value(), 3);
    }

    // --- WiFi URI ---

    #[test]
    fn build_wifi_uri_wpa() {
        let args = Args {
            wifi: true,
            ssid: Some("Home".to_string()),
            password: Some("secret".to_string()),
            wifitype: WifiType::Wpa,
            vcard: false,
            batch: None,
            text: String::new(),
            output: None,
            stdin: false,
            stdout: false,
            source_file: None,
            name: None,
            phone: None,
            email: None,
            org: None,
            title: None,
            url: None,
            format: OutputFormat::Svg,
            term: false,
            size: None,
            shape: ModuleShape::Square,
            fg_color: "#000000".to_string(),
            bg_color: "#FFFFFF".to_string(),
            dark_mode: false,
            border: 4,
            ecc: "M".to_string(),
            mask: None,
            logo: None,
        };
        let uri = build_wifi_uri(&args).unwrap();
        assert_eq!(uri, "WIFI:T:WPA;S:Home;P:secret;;");
    }

    #[test]
    fn build_wifi_uri_wep() {
        let args = Args {
            wifi: true,
            ssid: Some("Net".to_string()),
            password: Some("pwd".to_string()),
            wifitype: WifiType::Wep,
            vcard: false,
            batch: None,
            text: String::new(),
            output: None,
            stdin: false,
            stdout: false,
            source_file: None,
            name: None,
            phone: None,
            email: None,
            org: None,
            title: None,
            url: None,
            format: OutputFormat::Svg,
            term: false,
            size: None,
            shape: ModuleShape::Square,
            fg_color: "#000000".to_string(),
            bg_color: "#FFFFFF".to_string(),
            dark_mode: false,
            border: 4,
            ecc: "M".to_string(),
            mask: None,
            logo: None,
        };
        let uri = build_wifi_uri(&args).unwrap();
        assert_eq!(uri, "WIFI:T:WEP;S:Net;P:pwd;;");
    }

    #[test]
    fn build_wifi_uri_open_network() {
        let args = Args {
            wifi: true,
            ssid: Some("Cafe".to_string()),
            password: None,
            wifitype: WifiType::None,
            vcard: false,
            batch: None,
            text: String::new(),
            output: None,
            stdin: false,
            stdout: false,
            source_file: None,
            name: None,
            phone: None,
            email: None,
            org: None,
            title: None,
            url: None,
            format: OutputFormat::Svg,
            term: false,
            size: None,
            shape: ModuleShape::Square,
            fg_color: "#000000".to_string(),
            bg_color: "#FFFFFF".to_string(),
            dark_mode: false,
            border: 4,
            ecc: "M".to_string(),
            mask: None,
            logo: None,
        };
        let uri = build_wifi_uri(&args).unwrap();
        assert_eq!(uri, "WIFI:T:nopass;S:Cafe;P:;;");
    }

    #[test]
    fn build_wifi_uri_escapes_reserved_characters() {
        let args = Args {
            wifi: true,
            ssid: Some(r#"Cafe;Floor:2, "Guest""#.to_string()),
            password: Some(r#"pa\ss;word"#.to_string()),
            wifitype: WifiType::Wpa,
            vcard: false,
            batch: None,
            text: String::new(),
            output: None,
            stdin: false,
            stdout: false,
            source_file: None,
            name: None,
            phone: None,
            email: None,
            org: None,
            title: None,
            url: None,
            format: OutputFormat::Svg,
            term: false,
            size: None,
            shape: ModuleShape::Square,
            fg_color: "#000000".to_string(),
            bg_color: "#FFFFFF".to_string(),
            dark_mode: false,
            border: 4,
            ecc: "M".to_string(),
            mask: None,
            logo: None,
        };
        let uri = build_wifi_uri(&args).unwrap();
        assert_eq!(
            uri,
            r#"WIFI:T:WPA;S:Cafe\;Floor\:2\, \"Guest\";P:pa\\ss\;word;;"#
        );
    }

    #[test]
    fn build_wifi_uri_missing_ssid() {
        let args = Args {
            wifi: true,
            ssid: None,
            password: Some("x".to_string()),
            wifitype: WifiType::Wpa,
            vcard: false,
            batch: None,
            text: String::new(),
            output: None,
            stdin: false,
            stdout: false,
            source_file: None,
            name: None,
            phone: None,
            email: None,
            org: None,
            title: None,
            url: None,
            format: OutputFormat::Svg,
            term: false,
            size: None,
            shape: ModuleShape::Square,
            fg_color: "#000000".to_string(),
            bg_color: "#FFFFFF".to_string(),
            dark_mode: false,
            border: 4,
            ecc: "M".to_string(),
            mask: None,
            logo: None,
        };
        assert!(build_wifi_uri(&args).is_err());
    }

    // --- vCard ---

    #[test]
    fn build_vcard_minimal() {
        let args = Args {
            vcard: true,
            name: Some("John".to_string()),
            phone: Some("123".to_string()),
            email: Some("john@example.com".to_string()),
            org: None,
            title: None,
            url: None,
            wifi: false,
            ssid: None,
            password: None,
            wifitype: WifiType::Wpa,
            batch: None,
            text: String::new(),
            output: None,
            stdin: false,
            stdout: false,
            source_file: None,
            format: OutputFormat::Svg,
            term: false,
            size: None,
            shape: ModuleShape::Square,
            fg_color: "#000000".to_string(),
            bg_color: "#FFFFFF".to_string(),
            dark_mode: false,
            border: 4,
            ecc: "M".to_string(),
            mask: None,
            logo: None,
        };
        let vcard = build_vcard(&args).unwrap();
        assert!(vcard.contains("BEGIN:VCARD"));
        assert!(vcard.contains("END:VCARD"));
        assert!(vcard.contains("FN:John"));
        assert!(vcard.contains("TEL:123"));
        assert!(vcard.contains("EMAIL:john@example.com"));
    }

    #[test]
    fn build_vcard_full() {
        let args = Args {
            vcard: true,
            name: Some("Jane".to_string()),
            phone: Some("+1234".to_string()),
            email: Some("jane@co.com".to_string()),
            org: Some("ACME".to_string()),
            title: Some("Engineer".to_string()),
            url: Some("https://acme.com".to_string()),
            wifi: false,
            ssid: None,
            password: None,
            wifitype: WifiType::Wpa,
            batch: None,
            text: String::new(),
            output: None,
            stdin: false,
            stdout: false,
            source_file: None,
            format: OutputFormat::Svg,
            term: false,
            size: None,
            shape: ModuleShape::Square,
            fg_color: "#000000".to_string(),
            bg_color: "#FFFFFF".to_string(),
            dark_mode: false,
            border: 4,
            ecc: "M".to_string(),
            mask: None,
            logo: None,
        };
        let vcard = build_vcard(&args).unwrap();
        assert!(vcard.contains("ORG:ACME"));
        assert!(vcard.contains("TITLE:Engineer"));
        assert!(vcard.contains("URL:https://acme.com"));
    }

    // --- SVG ---

    #[test]
    fn qr_to_svg_contains_xml_declaration() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(
            &qr,
            4,
            "#000000",
            "#FFFFFF",
            &ModuleShape::Square,
            None,
            &None,
        );
        assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(svg.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }

    #[test]
    fn qr_to_svg_contains_version_comment() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(
            &qr,
            4,
            "#000000",
            "#FFFFFF",
            &ModuleShape::Square,
            None,
            &None,
        );
        assert!(svg.contains("<!-- generated by flexq"));
    }

    #[test]
    fn qr_to_svg_contains_colors() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(
            &qr,
            4,
            "#FF0000",
            "#FFFFCC",
            &ModuleShape::Square,
            None,
            &None,
        );
        assert!(svg.contains("fill=\"#FFFFCC\""));
        assert!(svg.contains("fill=\"#FF0000\""));
    }

    #[test]
    fn qr_to_svg_border_affects_viewbox() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let size = qr.size();

        let svg_b0 = qr_to_svg(&qr, 0, "#000", "#FFF", &ModuleShape::Square, None, &None);
        assert!(svg_b0.contains(&format!(r#"viewBox="0 0 {} {}""#, size, size)));

        let svg_b8 = qr_to_svg(&qr, 8, "#000", "#FFF", &ModuleShape::Square, None, &None);
        let dim_b8 = size + 16;
        assert!(svg_b8.contains(&format!(r#"viewBox="0 0 {} {}""#, dim_b8, dim_b8)));
    }

    #[test]
    fn qr_to_svg_fixed_size() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(
            &qr,
            4,
            "#000",
            "#FFF",
            &ModuleShape::Square,
            Some(400),
            &None,
        );
        assert!(svg.contains(r#"viewBox="0 0 400 400""#));
        assert!(svg.contains(r#"width="400" height="400""#));
    }

    #[test]
    fn qr_to_svg_contains_crisp_edges() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(
            &qr,
            4,
            "#000000",
            "#FFFFFF",
            &ModuleShape::Square,
            None,
            &None,
        );
        assert!(svg.contains(r#"shape-rendering="crispEdges""#));
    }

    #[test]
    fn qr_to_svg_circle_shape() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(&qr, 4, "#000", "#FFF", &ModuleShape::Circle, None, &None);
        assert!(svg.contains("<circle"));
        assert!(!svg.contains("<path d="));
    }

    #[test]
    fn qr_to_svg_rounded_shape() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(&qr, 4, "#000", "#FFF", &ModuleShape::Rounded, None, &None);
        assert!(svg.contains("<rect"));
        assert!(svg.contains("rx="));
    }

    #[test]
    fn qr_to_svg_hexagon_shape() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let svg = qr_to_svg(&qr, 4, "#000", "#FFF", &ModuleShape::Hexagon, None, &None);
        assert!(svg.contains("<polygon"));
    }

    // --- PNG ---

    #[test]
    fn qr_to_png_produces_valid_png() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let png = qr_to_png(&qr, 4, "#000000", "#FFFFFF", None).unwrap();
        // PNG magic bytes
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn qr_to_png_respects_colors() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let _png = qr_to_png(&qr, 4, "#FF0000", "#FFFF00", None).unwrap();
        // If it doesn't panic, the colors parsed correctly
    }

    #[test]
    fn qr_to_png_fixed_size() {
        let qr = QrCode::encode_text("test", QrCodeEcc::Medium).unwrap();
        let png = qr_to_png(&qr, 4, "#000", "#FFF", Some(400)).unwrap();
        let img = image::load_from_memory(&png).unwrap();

        assert_eq!(img.width(), 400);
        assert_eq!(img.height(), 400);
    }

    // --- RGB parsing ---

    #[test]
    fn parse_rgb_3_digit() {
        let [r, g, b] = parse_rgb("#F00").unwrap();
        assert_eq!([r, g, b], [255, 0, 0]);
    }

    #[test]
    fn parse_rgb_6_digit() {
        let [r, g, b] = parse_rgb("#00FF00").unwrap();
        assert_eq!([r, g, b], [0, 255, 0]);
    }

    #[test]
    fn parse_rgb_rejects_no_hash() {
        assert!(parse_rgb("FFF").is_err());
    }

    // --- Base64 ---

    #[test]
    fn base64_encode_known_value() {
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"M"), "TQ==");
        assert_eq!(base64_encode(b""), "");
    }
}
