# FlexQ

A minimal command-line QR code generator written in Rust. Encodes arbitrary text or URLs into standalone SVG files.

## Features

- **Lightweight** — single binary with minimal dependencies
- **SVG output** — resolution-independent, opens in any browser or vector editor
- **Flexible input** — encode from argument, stdin, or file
- **Flexible output** — save to file or pipe to stdout
- **Color validation** — rejects invalid hex colors upfront
- **Medium error correction** — QR codes remain scannable even if partially damaged
- **Customizable** — configurable border size, foreground color, and background color

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
flexq [TEXT] [OUTPUT] [OPTIONS]
```

Both `TEXT` and `OUTPUT` are optional — use `--stdin` or `--source-file` for input, and `--stdout` for output.

### Arguments

| Argument   | Description                                      |
|------------|--------------------------------------------------|
| `[TEXT]`   | The text or URL to encode into a QR code. |
| `[OUTPUT]` | The file path where the SVG will be saved. |

### Options

| Option             | Short  | Default    | Description                                          |
|--------------------|--------|------------|------------------------------------------------------|
| `--stdin`          | `-i`   | —          | Read the text to encode from standard input.         |
| `--stdout`         | `-o`   | —          | Write the SVG QR code to standard output.            |
| `--source-file <F>`| `-s`   | —          | Read the text to encode from a file.                 |
| `--border <N>`     | `-b`   | `4`        | Border size in QR modules around the code.           |
| `--fg-color <C>`   | `-F`   | `#000000`  | Foreground (module) color of the QR code (hex).      |
| `--bg-color <C>`   | `-B`   | `#FFFFFF`  | Background color of the QR code (hex).               |

### Examples

Generate a QR code for a URL:

```bash
flexq "https://example.com" qrcode.svg
```

Generate a QR code for plain text:

```bash
flexq "Hello, world!" hello.svg
```

Read from stdin and write to stdout (pipe-friendly):

```bash
echo "https://example.com" | flexq --stdin --stdout > qrcode.svg
```

Read from a file:

```bash
flexq --source-file input.txt output.svg
```

Generate with a larger border:

```bash
flexq "https://example.com" qrcode.svg --border 8
```

Generate with custom colors:

```bash
flexq "https://example.com" qrcode.svg --fg-color "#FF0000" --bg-color "#FFFFCC"
```

Pipe the result directly to another tool:

```bash
flexq "https://example.com" --stdout | xdg-open -
```

### Help

```bash
flexq -h
# or
flexq --help
```

## Output format

The generated SVG is self-contained with:

- A background rectangle (default: white, customizable via `--bg-color`)
- QR modules rendered as a single `<path>` element (default: black, customizable via `--fg-color`)
- A configurable border (quiet zone) around the code (default: 4 modules, set via `--border`)
- `shape-rendering="crispEdges"` for sharp pixel-perfect rendering

## License

Licensed under either the [MIT license](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option.

## Contributing

Issues and pull requests are welcome. Feel free to open an issue for feature requests or bug reports.
