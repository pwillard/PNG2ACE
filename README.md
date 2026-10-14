# png2ace

A Rust command-line converter for PNG/TGA images to Microsoft Train Simulator ACE texture files.

Current implementation writes uncompressed, structured ACE files with 8-bit red, green, and blue channels. It detects source alpha channels, but only emits an ACE alpha plane when transparency appears meaningful; tiny amounts of near-opaque alpha noise are treated as opaque because some legacy MSTS ACE readers misread unnecessary alpha planes as image data. It targets the format consumed by Microsoft Train Simulator/Open Rails and deliberately starts with the simplest encoder before adding compression and legacy packed formats.

## Usage

```text
cargo run --release --
cargo run --release -- folder
cargo run --release -- folder output-folder
cargo run --release -- input.png output.ace
cargo run --release -- input.tga output.ace
cargo run --release -- input.png output.ace --mipmaps
cargo run --release -- input.png output.ace --format dxt1
cargo run --release -- folder output-folder --format auto-dxt
```

Or use the compiled binary:

```text
./target/release/png2ace
./target/release/png2ace folder
./target/release/png2ace input.png output.ace
./target/release/png2ace input.tga output.ace
```

When no input is provided, `png2ace` converts every `.png`, `.tga`, or `.targa` file in the current folder to a same-name `.ace` file. Folder conversion is non-recursive and matches extensions case-insensitively. If two inputs would produce the same output name, such as `texture.png` and `texture.tga`, conversion is rejected instead of overwriting one result.

## GUI

Build or run the cross-platform GUI front end with:

```text
cargo run --release --bin png2ace-gui
```

The GUI supports single-image conversion, folder batch conversion, native file/folder browse dialogs, a mipmaps checkbox, and a status log. The CLI remains available as `png2ace` for scripts and batch use.

See `docs/PNG2ACE_User_Guide.md` for full command-line and GUI usage instructions.

## ACE output formats

The default output is uncompressed structured ACE data. DXT/raw ACE output is also available:

```text
--format structured
--format dxt1
--format dxt3
--format dxt5
--format auto-dxt
```

`auto-dxt` writes DXT1 for opaque images and DXT5 when meaningful transparency is detected. DXT3 and DXT5 are alpha-image formats; if either is requested for a PNG/TGA source with no alpha channel, the file is skipped and no ACE is written. DXT output uses ACE raw surface data with MSTS/Open Rails surface IDs `0x12` for DXT1, `0x14` for DXT3, and `0x16` for DXT5.

## Current limitations

- Mipmap generation is supported with `--mipmaps` for square power-of-two images.
- No `SIMISA@F` zlib-compressed envelope.
- No RGB565, BGRA5551, or BGRA4444 output.
- Output has not yet been tested in a running MSTS/Open Rails installation.

## Format implementation

The encoder writes the uncompressed `SIMISA@@@@@@@@@@` envelope and the structured channel layout used by the Open Rails ACE reader. Structured rows are written as planar channel data: red row bytes, green row bytes, blue row bytes, then alpha row bytes when the source image has meaningful transparency.

The format reference used during development is Open Rails' `AceFile.cs`. This project does not copy that source file.
