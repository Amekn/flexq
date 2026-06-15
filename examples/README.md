# Examples

Sample QR codes generated with FlexQ. Each file encodes the URL listed below.

| File | URL |
|------|-----|
| `google.svg` | https://www.google.com |
| `github.svg` | https://www.github.com |
| `rust.svg` | https://www.rust-lang.org |
| `crates_io.svg` | https://crates.io |
| `youtube.svg` | https://www.youtube.com |
| `wikipedia.svg` | https://en.wikipedia.org |

## Regenerating

```bash
cargo build --release
./target/release/flexq "https://www.google.com" -o examples/google.svg
./target/release/flexq "https://www.github.com" -o examples/github.svg
```

### With styling

```bash
./target/release/flexq "https://www.rust.org" --shape circle --fg-color "#CE422B" -o examples/rust.svg
./target/release/flexq "https://crates.io" --shape rounded --size 400 -o examples/crates_io.svg
```

## Scanning

Open any SVG in a browser or scan with a QR code reader app to verify the encoded URL.
