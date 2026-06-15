# FlexQ

A flexible command-line QR code generator written in Rust. Encodes arbitrary text, URLs, WiFi credentials, or vCard contacts into standalone SVG or PNG files.

## Features

- **Lightweight** — single binary with minimal dependencies
- **SVG & PNG output** — resolution-independent vector or raster images
- **Flexible input** — encode from argument, stdin, or file
- **Flexible output** — save to file, pipe to stdout, or preview in terminal
- **WiFi QR codes** — generate scannable WiFi configuration codes
- **vCard QR codes** — generate contact cards scannable by any phone
- **Module shapes** — square, rounded, circle, or hexagon module styles
- **Logo overlay** — embed a logo in the center of the QR code
- **Error correction** — configurable L/M/Q/H levels
- **Dark mode** — auto-invert colors for dark backgrounds
- **Batch mode** — generate multiple QR codes from a CSV/TSV file
- **Color validation** — rejects invalid hex colors with helpful error messages
- **Terminal preview** — verify QR codes in the console before saving

## Installation

### From source

```bash
git clone https://github.com/Amekn/flexq.git
cd flexq
cargo build --release
```

The binary will be at `target/release/flexq`.

### From crates.io

```bash
cargo install flexq
```

## Usage

```
flexq [TEXT] [OPTIONS]
```

### Arguments

| Argument   | Description                                      |
|------------|--------------------------------------------------|
| `[TEXT]`   | The text or URL to encode into a QR code. |

### Options

| Option               | Short  | Default    | Description                                              |
|----------------------|--------|------------|----------------------------------------------------------|
| `-o`, `--output <F>` | `-o`   | —          | File path where the QR code will be saved.               |
| `--stdin`            | `-i`   | —          | Read the text to encode from standard input.             |
| `--stdout`           | —      | —          | Write the QR code to standard output.                    |
| `--source-file <F>`  | `-s`   | —          | Read the text to encode from a file.                     |
| `--format <F>`       | —      | `svg`      | Output format: `svg` or `png`.                           |
| `--term`             | —      | —          | Print QR code to terminal using Unicode block characters. No file written unless `-o` is also provided. |
| `--size <N>`         | —      | auto       | Fixed output size in pixels (default: module-based).     |
| `--shape <S>`        | —      | `square`   | Module shape: `square`, `rounded`, `circle`, `hexagon`.  |
| `--fg-color <C>`     | `-F`   | `#000000`  | Foreground (module) color in hex.                        |
| `--bg-color <C>`     | `-B`   | `#FFFFFF`  | Background color in hex.                                 |
| `--dark-mode`        | —      | —          | Auto-invert fg/bg colors for dark backgrounds.           |
| `-b`, `--border <N>` | `-b`   | `4`        | Border size in QR modules around the code.               |
| `--ecc <L>`          | —      | `M`        | Error correction level: `L`, `M`, `Q`, `H`.              |
| `--mask <N>`         | —      | auto       | Mask pattern 0–7 (usually auto is fine).                 |
| `--logo <F>`         | —      | —          | Overlay a logo image in the center of the QR code.       |
| `--batch <F>`        | —      | —          | Path to CSV/TSV file with `text,output_path` rows.       |

### WiFi Options

| Option             | Description                                    |
|--------------------|------------------------------------------------|
| `--wifi`           | Generate a WiFi configuration QR code.         |
| `--ssid <NAME>`    | WiFi network name (required with `--wifi`).    |
| `--password <P>`   | WiFi password.                                 |
| `--wifitype <T>`   | Encryption type: `wpa`, `wep`, `none` (default: `wpa`). |

### vCard Options

| Option             | Description                                    |
|--------------------|------------------------------------------------|
| `--vcard`          | Generate a vCard contact QR code.              |
| `--name <N>`       | Contact full name.                             |
| `--phone <P>`      | Contact phone number.                          |
| `--email <E>`      | Contact email address.                         |
| `--org <O>`        | Contact organization.                          |
| `--title <T>`      | Contact job title.                             |
| `--url <U>`        | Contact website URL.                           |

## Examples

### Basic QR code

```bash
flexq "https://example.com" -o qrcode.svg
```

### Plain text

```bash
flexq "Hello, world!" -o hello.svg
```

### Terminal preview (no file written)

```bash
flexq "https://example.com" --term
```

### Terminal preview + save

```bash
flexq "https://example.com" --term -o qrcode.svg
```

### PNG output with fixed size

```bash
flexq "https://example.com" --format png --size 400 -o qrcode.png
```

### Custom shape and colors

```bash
flexq "https://example.com" --shape circle --fg-color "#FF6B6B" --bg-color "#1A1A2E" -o styled.svg
```

### Dark mode

```bash
flexq "https://example.com" --dark-mode -o dark.svg
```

### WiFi network (WPA)

```bash
flexq --wifi --ssid "HomeNetwork" --password "mySecret123" --wifitype wpa -o wifi.svg
```

### WiFi network (open)

```bash
flexq --wifi --ssid "CafeWiFi" --wifitype none -o wifi.svg
```

### vCard contact

```bash
flexq --vcard --name "John Doe" --phone "123-456-7890" --email "john@example.com" -o contact.svg
```

### Full vCard

```bash
flexq --vcard \
  --name "Jane Smith" \
  --phone "+44-20-7946-0958" \
  --email "jane@corp.uk" \
  --org "ACME Ltd" \
  --title "Engineer" \
  --url "https://acme.uk" \
  -o jane.svg
```

### Branded QR code with logo

```bash
flexq "https://example.com" --logo company-logo.png --ecc H --size 500 -o branded.svg
```

### High error correction

```bash
flexq "https://example.com" --ecc H -o resilient.svg
```

### Batch mode

Create `codes.csv`:

```csv
https://example.com/page1,output/page1.svg
https://example.com/page2,output/page2.svg
https://example.com/page3,output/page3.svg
```

Then run:

```bash
flexq --batch codes.csv --shape rounded --size 400
```

### Pipe-friendly (stdin → stdout)

```bash
echo "https://example.com" | flexq --stdin --stdout > qrcode.svg
```

### Read from file

```bash
flexq --source-file input.txt -o output.svg
```

## Output format

### SVG

The generated SVG is self-contained with:

- A version comment (`<!-- generated by flexq 0.2.1 -->`)
- A background rectangle (customizable via `--bg-color`)
- QR modules rendered as paths or shapes depending on `--shape`
- Configurable border (quiet zone) around the code
- `shape-rendering="crispEdges"` for sharp rendering
- Optional logo overlay with embedded base64 image

### PNG

Raster output with:

- Fixed pixel dimensions (set via `--size`, defaults to module-based)
- Configurable foreground and background colors
- Clean edges suitable for embedding in documents

## License

Licensed under either the [MIT license](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option.

## Contributing

Issues and pull requests are welcome. Feel free to open an issue for feature requests or bug reports.
