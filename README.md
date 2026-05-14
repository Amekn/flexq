# FlexQ

A minimal command-line QR code generator written in Rust. Encodes arbitrary text or URLs into standalone SVG files.

## Features

- **Lightweight** — single binary with minimal dependencies
- **SVG output** — resolution-independent, opens in any browser or vector editor
- **Medium error correction** — QR codes remain scannable even if partially damaged
- **Simple interface** — two arguments and you're done

## Installation

### From source

```bash
git clone https://github.com/Aemkn/flexq.git
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
flexq <text> <output.svg>
```

| Argument     | Description                                      |
|-------------|--------------------------------------------------|
| `<text>`    | The text or URL to encode into a QR code.        |
| `<output.svg>` | The file path where the SVG will be saved.  |

### Examples

Generate a QR code for a URL:

```bash
flexq "https://example.com" qrcode.svg
```

Generate a QR code for plain text:

```bash
flexq "Hello, world!" hello.svg
```

### Help

```bash
flexq -h
# or
flexq --help
```

## Output format

The generated SVG is self-contained with:

- A white background rectangle
- Black QR modules rendered as a single `<path>` element
- A 4-module border (quiet zone) around the code
- `shape-rendering="crispEdges"` for sharp pixel-perfect rendering

## License

Licensed under either the [MIT license](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option.

## Contributing

Issues and pull requests are welcome. Feel free to open an issue for feature requests or bug reports.
