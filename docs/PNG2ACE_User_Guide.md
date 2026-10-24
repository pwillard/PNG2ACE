# PNG2ACE User Guide

Version: 1.2.0

PNG2ACE converts PNG, TGA, and TARGA image files into Microsoft Train Simulator / Open Rails ACE texture files. It includes both a command-line program for batch work and a graphical interface for normal desktop use.

## Programs included

| Program | Use |
| --- | --- |
| `png2ace.exe` | Command-line converter for one file or a folder batch. |
| `png2ace-gui.exe` | Graphical converter with file and folder browse buttons. |

## Supported input files

PNG2ACE accepts these input image types:

- `.png`
- `.tga`
- `.targa`

Folder batch conversion is non-recursive. It converts files in the selected folder only, not subfolders.

## ACE output formats

| Format | Best use | Alpha handling |
| --- | --- | --- |
| Structured RGB/RGBA | Safest default for MSTS/Open Rails compatibility. | Writes RGB unless the source has meaningful transparency, then RGB + 1-bit mask + 8-bit alpha (the MakeACE layout). |
| DXT1 / BC1 | Compressed textures without alpha, or with simple cut-out transparency. | Best compressed choice for opaque textures. Transparent images get a 1-bit mask, like MakeACE `-dxt -trans`. |
| DXT3 / BC2 (for alpha images) | Compressed textures with explicit alpha. | Requires a source image with an alpha channel. |
| DXT5 / BC3 (for alpha images) | Compressed textures with interpolated alpha. | Requires a source image with an alpha channel. |
| Auto DXT | Automatic compressed output. | Uses DXT1 for opaque images and DXT5 for meaningful transparency. |

### Alpha-channel rules

PNG2ACE checks whether the source image actually contains an alpha channel before deciding how to write the ACE file.

- Images with no alpha channel are treated as opaque.
- Images with a fully opaque alpha channel are treated as opaque for Structured and DXT1 output.
- Tiny near-opaque alpha noise is treated as opaque. This avoids striped or interlaced-looking textures in older ACE readers.
- Meaningful transparency produces an alpha-capable ACE format when the chosen output format supports alpha.
- Structured alpha output stores red, green, blue, a 1-bit mask and 8-bit alpha, exactly like Kuju's MakeACE. A mask pixel is solid when its alpha is above 32.
- If DXT3 or DXT5 is selected for an image with no alpha channel, PNG2ACE skips that file and prints/logs a message. No ACE file is written for that skipped image.

## Using the graphical interface

Start the GUI by running:

`png2ace-gui.exe`

The main window contains:

- Input mode: choose `Single image` or `Folder batch`.
- Source: the PNG/TGA image or folder to convert.
- Output: the ACE file or output folder. This can be left blank for the default output location.
- Generate mipmaps: optional mipmap-chain generation.
- ACE format: output format selection.
- Convert: starts conversion.
- Clear log: clears the status log.
- Status: progress, success, skip, and error messages.

### Convert one image in the GUI

1. Select `Single image`.
2. Click the Source `Browse...` button.
3. Pick a `.png`, `.tga`, or `.targa` file.
4. Leave Output blank to create an ACE file beside the source image, or click Output `Browse...` to choose a different ACE filename.
5. Choose the ACE format.
6. Optional: enable `Generate mipmaps` if the image is square and power-of-two.
7. Click `Convert`.
8. Read the Status log for the result.

### Convert a folder in the GUI

1. Select `Folder batch`.
2. Click the Source `Browse...` button and choose a folder.
3. Leave Output blank to write ACE files into the same folder, or choose a different output folder.
4. Choose the ACE format.
5. Optional: enable `Generate mipmaps` if every image is square and power-of-two.
6. Click `Convert`.
7. The Status log shows each file as it is processed.

### GUI format choices

Use these practical defaults:

- Use `Structured RGB/RGBA` when compatibility matters most.
- Use `DXT1 / BC1` for opaque compressed textures.
- Use `DXT3 / BC2 (for alpha images)` or `DXT5 / BC3 (for alpha images)` only when the source image has an alpha channel.
- Use `Auto DXT` when you want compressed output and want PNG2ACE to choose DXT1 or DXT5 automatically.

## Using the command-line program

Open a Command Prompt, PowerShell window, or terminal in the folder containing `png2ace.exe`.

Basic command form:

`png2ace.exe [OPTIONS] [INPUT] [OUTPUT]`

### Command-line arguments

| Argument | Meaning |
| --- | --- |
| `INPUT` | Source PNG/TGA image or folder. If omitted, the current folder is used. |
| `OUTPUT` | Destination ACE file for one image, or destination folder for folder conversion. |

### Command-line options

| Option | Meaning |
| --- | --- |
| `--format structured` | Write uncompressed Structured ACE output. This is the default. |
| `--format dxt1` | Write DXT1 / BC1 compressed ACE output. |
| `--format dxt3` | Write DXT3 / BC2 compressed ACE output for alpha images. Skips no-alpha sources. |
| `--format dxt5` | Write DXT5 / BC3 compressed ACE output for alpha images. Skips no-alpha sources. |
| `--format auto-dxt` | Automatically choose DXT1 or DXT5. |
| `--mipmaps` | Generate a complete mipmap chain. Requires square power-of-two images. |
| `-h`, `--help` | Show command help. |
| `-V`, `--version` | Show the program version. |

### Command-line examples

Convert all PNG/TGA files in the current folder using Structured ACE output:

`png2ace.exe`

Convert all PNG/TGA files in a folder:

`png2ace.exe C:\Textures`

Convert all PNG/TGA files in one folder and write ACE files to another folder:

`png2ace.exe C:\Textures C:\Textures\ACE`

Convert one image beside the source file:

`png2ace.exe 90tex.png`

Convert one image to a specific ACE file:

`png2ace.exe 90tex.png 90tex.ace`

Convert one TGA image:

`png2ace.exe texture.tga texture.ace`

Convert one image to DXT1:

`png2ace.exe texture.png texture.ace --format dxt1`

Convert a folder to Auto DXT:

`png2ace.exe C:\Textures C:\Textures\ACE --format auto-dxt`

Convert one image with mipmaps:

`png2ace.exe texture.png texture.ace --mipmaps`

Convert one image to DXT1 with mipmaps:

`png2ace.exe texture.png texture.ace --format dxt1 --mipmaps`

### Folder conversion behavior

When converting a folder:

- PNG2ACE scans only the selected folder.
- Subfolders are not scanned.
- File extensions are matched case-insensitively.
- Output files use the same base name and the `.ace` extension.
- If two inputs would produce the same ACE filename, conversion is rejected before writing output. Example: `texture.png` and `texture.tga` would both produce `texture.ace`.
- Progress is printed as each file starts and finishes.

## Mipmaps

Mipmaps are smaller versions of the texture stored in the ACE file. They can improve rendering at a distance.

Mipmaps require:

- square images
- power-of-two dimensions, such as 256x256, 512x512, 1024x1024, or 2048x2048

If `--mipmaps` is used on an invalid image size, conversion fails for that image.

## Recommended workflows

### Safest general-purpose workflow

Use Structured output first:

`png2ace.exe texture.png texture.ace --format structured`

This is the safest choice when checking a new texture or when compatibility is more important than file size.

### Compressed opaque textures

Use DXT1:

`png2ace.exe texture.png texture.ace --format dxt1`

DXT1 is usually the right compressed choice for normal opaque textures.

### Compressed alpha textures

Use DXT5 or Auto DXT for textures with meaningful transparency:

`png2ace.exe window.png window.ace --format dxt5`

or:

`png2ace.exe window.png window.ace --format auto-dxt`

If the source file has no alpha channel, DXT3 and DXT5 are skipped because they are alpha-image formats.

## Status and messages

Typical successful single-file output:

`converting texture.png -> texture.ace ... done (1024x1024, 3 channel structured, no mipmaps)`

Typical skip message for DXT5 on a no-alpha image:

`converting texture.png -> texture.ace ... skipped (skipped: texture.png has no alpha channel; DXT5 is for alpha images)`

Typical folder output:

`found 3 PNG/TGA file(s) in C:\Textures`

`[1/3] converting C:\Textures\a.png -> C:\Textures\a.ace ... done (512x512, 3 channel DXT1, no mipmaps)`

## Troubleshooting

### The output ACE looks striped or interlaced

Use PNG2ACE 1.2 or later. Earlier versions wrote the ACE row table and alpha channels in a slightly different layout from Kuju's MakeACE, which some older ACE readers can show as stripes. 1.2 writes the same layout as MakeACE. PNG2ACE also avoids writing alpha planes for fully opaque alpha channels and tiny near-opaque alpha noise.

If you still see stripes, reconvert the texture from the original PNG/TGA with the current build; ACE files made by older PNG2ACE versions are not fixed automatically.

### DXT3 or DXT5 says the image was skipped

The input image does not contain an alpha channel. Use DXT1 or Structured output, or save the source image with an alpha channel if alpha output is really needed.

### Mipmap conversion fails

Check that the image is square and power-of-two. Examples that work: 256x256, 512x512, 1024x1024, 2048x2048.

### Folder conversion rejects duplicate output names

Check for files with the same base name but different extensions, such as:

- `texture.png`
- `texture.tga`

Both would write `texture.ace`, so PNG2ACE refuses the batch. Rename one source file or convert them to different output folders.

### The GUI executable cannot be replaced during an update

Close `png2ace-gui.exe` before copying a new build over it. Windows locks running executables.

## Quick reference

| Task | GUI | Command line |
| --- | --- | --- |
| Convert one image | Single image, Browse source, Convert | `png2ace.exe image.png` |
| Convert current folder | Folder batch, Source `.` or chosen folder, Convert | `png2ace.exe` |
| Convert folder to another folder | Folder batch, choose Source and Output folders | `png2ace.exe input-folder output-folder` |
| Use DXT1 | Select `DXT1 / BC1` | `--format dxt1` |
| Use DXT3 | Select `DXT3 / BC2 (for alpha images)` | `--format dxt3` |
| Use DXT5 | Select `DXT5 / BC3 (for alpha images)` | `--format dxt5` |
| Let PNG2ACE choose DXT | Select `Auto DXT` | `--format auto-dxt` |
| Generate mipmaps | Check `Generate mipmaps` | `--mipmaps` |

## Notes for Open Rails and MSTS users

PNG2ACE is designed for ACE textures used by Microsoft Train Simulator and Open Rails. Structured output is the most conservative choice. DXT output is smaller and may be useful for distribution or large texture sets, but test important textures in your target simulator or viewer before replacing working originals.

### Compatibility with MakeACE

PNG2ACE 1.2 writes ACE files in the same layout as Kuju's original MakeACE 1.04: the same header values, channel list, row table and full-size pixel data. The differences you can expect are:

- Mipmaps: MakeACE copies one pixel from each 2x2 block; PNG2ACE averages the block, so its distant mipmaps look smoother.
- Alpha detection: MakeACE treats any alpha below 255 as transparency; PNG2ACE ignores tiny near-opaque alpha noise.
- PNG2ACE does not write zlib-compressed (`SIMISA@F`) ACE files.
