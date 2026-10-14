use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use image::ImageReader;
use texpresso::{Algorithm, Format, Params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    /// Pixels in row-major RGBA8 order.
    pub pixels: Vec<[u8; 4]>,
    /// Whether the original source image contained an alpha channel before RGBA conversion.
    ///
    /// Fully opaque alpha channels are tracked but not emitted as ACE alpha planes; some legacy
    /// MSTS ACE readers misread an unused opaque alpha plane as image data.
    pub source_has_alpha: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AceOptions {
    pub mipmaps: bool,
    pub format: AceFormat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConversionOptions {
    pub mipmaps: bool,
    pub format: AceFormat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AceFormat {
    #[default]
    Structured,
    Dxt1,
    Dxt3,
    Dxt5,
    AutoDxt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Structured,
    Dxt1,
    Dxt3,
    Dxt5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionReport {
    pub input: PathBuf,
    pub output: PathBuf,
    pub width: u32,
    pub height: u32,
    pub channel_count: u32,
    pub format: &'static str,
    pub skipped: bool,
    pub message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AceError {
    #[error("texture dimensions must be non-zero")]
    EmptyTexture,
    #[error("pixel count does not match texture dimensions")]
    PixelCountMismatch,
    #[error("mipmaps require a square power-of-two texture")]
    InvalidMipmaps,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn convert_image_file(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    options: ConversionOptions,
) -> Result<ConversionReport, Box<dyn std::error::Error>> {
    let input = input.as_ref();
    let output = output.as_ref();
    let decoded = ImageReader::open(input)?.decode()?;
    let source_has_alpha = decoded.color().has_alpha();
    let image = decoded.to_rgba8();
    let (width, height) = image.dimensions();
    let pixels = image.pixels().map(|pixel| pixel.0).collect();
    let texture = Texture {
        width,
        height,
        pixels,
        source_has_alpha,
    };
    let output_format = resolve_output_format(options.format, &texture);
    let channel_count = output_channel_count(output_format, &texture);

    if alpha_format_requires_source_alpha(options.format) && !source_has_alpha {
        return Ok(ConversionReport {
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            width,
            height,
            channel_count,
            format: output_format.name(),
            skipped: true,
            message: Some(format!(
                "skipped: {} has no alpha channel; {} is for alpha images",
                input.display(),
                output_format.name()
            )),
        });
    }

    let mut output_file = BufWriter::new(File::create(output)?);
    write_ace_with_options(
        &texture,
        &mut output_file,
        AceOptions {
            mipmaps: options.mipmaps,
            format: options.format,
        },
    )?;

    Ok(ConversionReport {
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        width,
        height,
        channel_count,
        format: output_format.name(),
        skipped: false,
        message: None,
    })
}

pub fn convert_image_folder_with_progress<F>(
    input_folder: impl AsRef<Path>,
    output_folder: Option<impl AsRef<Path>>,
    options: ConversionOptions,
    mut progress: F,
) -> Result<Vec<ConversionReport>, Box<dyn std::error::Error>>
where
    F: FnMut(usize, usize, &Path, &Path),
{
    let input_folder = input_folder.as_ref();
    let output_folder = output_folder
        .as_ref()
        .map(|path| path.as_ref())
        .unwrap_or(input_folder);
    fs::create_dir_all(output_folder)?;

    let images = collect_image_files(input_folder)?;
    validate_unique_outputs(output_folder, &images)?;
    let mut reports = Vec::with_capacity(images.len());
    for (index, image) in images.iter().enumerate() {
        let output = image_output_path(output_folder, image)?;
        progress(index + 1, images.len(), image, &output);
        reports.push(convert_image_file(image, &output, options)?);
    }
    Ok(reports)
}

pub fn collect_image_files(folder: impl AsRef<Path>) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut images = fs::read_dir(folder)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    images.retain(|path| path.is_file() && is_supported_image(path));
    images.sort();
    Ok(images)
}

pub fn is_supported_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "tga" | "targa")
    )
}

pub fn image_output_path(
    output_folder: &Path,
    input: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(output_folder
        .join(
            input
                .file_name()
                .ok_or_else(|| format!("missing file name for {}", input.display()))?,
        )
        .with_extension("ace"))
}

pub fn validate_unique_outputs(
    output_folder: &Path,
    inputs: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut seen = HashSet::new();
    for input in inputs {
        let output = image_output_path(output_folder, input)?;
        let key = output.display().to_string().to_ascii_lowercase();
        if !seen.insert(key) {
            return Err(format!(
                "multiple input files would write the same output ACE: {}",
                output.display()
            )
            .into());
        }
    }
    Ok(())
}

pub fn write_ace<W: Write>(texture: &Texture, output: W) -> Result<(), AceError> {
    write_ace_with_options(texture, output, AceOptions::default())
}

impl OutputFormat {
    fn name(self) -> &'static str {
        match self {
            OutputFormat::Structured => "structured",
            OutputFormat::Dxt1 => "DXT1",
            OutputFormat::Dxt3 => "DXT3",
            OutputFormat::Dxt5 => "DXT5",
        }
    }

    fn surface_format(self) -> u32 {
        match self {
            OutputFormat::Structured => 0,
            OutputFormat::Dxt1 => 0x12,
            OutputFormat::Dxt3 => 0x14,
            OutputFormat::Dxt5 => 0x16,
        }
    }

    fn texpresso_format(self) -> Option<Format> {
        match self {
            OutputFormat::Structured => None,
            OutputFormat::Dxt1 => Some(Format::Bc1),
            OutputFormat::Dxt3 => Some(Format::Bc2),
            OutputFormat::Dxt5 => Some(Format::Bc3),
        }
    }
}

fn resolve_output_format(format: AceFormat, texture: &Texture) -> OutputFormat {
    match format {
        AceFormat::Structured => OutputFormat::Structured,
        AceFormat::Dxt1 => OutputFormat::Dxt1,
        AceFormat::Dxt3 => OutputFormat::Dxt3,
        AceFormat::Dxt5 => OutputFormat::Dxt5,
        AceFormat::AutoDxt => {
            if uses_meaningful_alpha(texture) {
                OutputFormat::Dxt5
            } else {
                OutputFormat::Dxt1
            }
        }
    }
}

fn alpha_format_requires_source_alpha(format: AceFormat) -> bool {
    matches!(format, AceFormat::Dxt3 | AceFormat::Dxt5)
}

fn output_channel_count(output_format: OutputFormat, texture: &Texture) -> u32 {
    match output_format {
        OutputFormat::Dxt3 | OutputFormat::Dxt5 => 4,
        OutputFormat::Structured | OutputFormat::Dxt1 => ace_channel_count(texture),
    }
}

pub fn ace_channel_count(texture: &Texture) -> u32 {
    if uses_meaningful_alpha(texture) { 4 } else { 3 }
}

fn uses_meaningful_alpha(texture: &Texture) -> bool {
    if !texture.source_has_alpha || texture.pixels.is_empty() {
        return false;
    }

    let mut non_opaque_pixels = 0usize;
    let mut minimum_alpha = 0xFFu8;
    for pixel in &texture.pixels {
        let alpha = pixel[3];
        if alpha != 0xFF {
            non_opaque_pixels += 1;
            minimum_alpha = minimum_alpha.min(alpha);
        }
    }

    minimum_alpha <= 0x7F || non_opaque_pixels * 100 >= texture.pixels.len()
}

pub fn write_ace_with_options<W: Write>(
    texture: &Texture,
    mut output: W,
    options: AceOptions,
) -> Result<(), AceError> {
    validate_texture(texture)?;
    if options.mipmaps && (texture.width != texture.height || !texture.width.is_power_of_two()) {
        return Err(AceError::InvalidMipmaps);
    }

    let output_format = resolve_output_format(options.format, texture);
    if output_format != OutputFormat::Structured {
        return write_raw_dxt_ace(texture, output, options, output_format);
    }

    let levels = if options.mipmaps {
        build_mipmaps(texture)
    } else {
        vec![texture.clone()]
    };
    let channel_count = ace_channel_count(texture);

    // Uncompressed ACE envelope followed by the structured texture header.
    output.write_all(b"SIMISA@@@@@@@@@@")?;
    output.write_all(&1u32.to_le_bytes())?;
    output.write_all(&(u32::from(options.mipmaps)).to_le_bytes())?;
    output.write_all(&texture.width.to_le_bytes())?;
    output.write_all(&texture.height.to_le_bytes())?;
    output.write_all(&0u32.to_le_bytes())?; // surface format is unused for structured data
    output.write_all(&channel_count.to_le_bytes())?; // red, green, blue, optional alpha
    output.write_all(&[0u8; 128])?;

    let channel_types: &[u64] = if channel_count == 4 {
        &[3, 4, 5, 6]
    } else {
        &[3, 4, 5]
    };
    for &channel_type in channel_types {
        output.write_all(&8u64.to_le_bytes())?; // 8 bits per channel
        output.write_all(&channel_type.to_le_bytes())?;
    }

    // Structured ACE stores one offset per scanline of every mipmap level.
    // Offsets are absolute file positions to the beginning of each row.
    let table_start = 16 + 4 + 5 * 4 + 128 + channel_count as usize * 16;
    let table_bytes: usize = levels.iter().map(|level| level.height as usize * 4).sum();
    let data_start = table_start + table_bytes;
    let mut next_offset = data_start;
    for level in &levels {
        let row_size = level.width as usize * channel_count as usize;
        for _ in 0..level.height {
            output.write_all(&(next_offset as u32).to_le_bytes())?;
            next_offset += row_size;
        }
    }

    // Structured ACE rows are planar: all red bytes, then green, blue, alpha.
    for level in &levels {
        for row in level.pixels.chunks_exact(level.width as usize) {
            for channel in 0..channel_count as usize {
                for pixel in row {
                    output.write_all(&[pixel[channel]])?;
                }
            }
        }
    }

    Ok(())
}

fn write_raw_dxt_ace<W: Write>(
    texture: &Texture,
    mut output: W,
    options: AceOptions,
    output_format: OutputFormat,
) -> Result<(), AceError> {
    let levels = if options.mipmaps {
        build_mipmaps(texture)
    } else {
        vec![texture.clone()]
    };
    let meaningful_alpha = uses_meaningful_alpha(texture);
    let compressed_levels: Vec<Vec<u8>> = levels
        .iter()
        .map(|level| compress_dxt_level(level, output_format, meaningful_alpha))
        .collect();

    output.write_all(b"SIMISA@@@@@@@@@@")?;
    output.write_all(&1u32.to_le_bytes())?;
    output.write_all(&(0x14u32 | u32::from(options.mipmaps)).to_le_bytes())?;
    output.write_all(&texture.width.to_le_bytes())?;
    output.write_all(&texture.height.to_le_bytes())?;
    output.write_all(&output_format.surface_format().to_le_bytes())?;
    let channel_count = output_channel_count(output_format, texture);
    output.write_all(&channel_count.to_le_bytes())?;
    output.write_all(&[0u8; 128])?;

    let channel_types: &[u64] = if channel_count == 4 {
        &[3, 4, 5, 6]
    } else {
        &[3, 4, 5]
    };
    for &channel_type in channel_types {
        output.write_all(&8u64.to_le_bytes())?;
        output.write_all(&channel_type.to_le_bytes())?;
    }

    let table_start = 16 + 4 + 5 * 4 + 128 + channel_count as usize * 16;
    let table_bytes = compressed_levels.len() * 4;
    let mut next_offset = table_start + table_bytes;
    for data in &compressed_levels {
        output.write_all(&(next_offset as u32).to_le_bytes())?;
        next_offset += 4 + data.len();
    }

    for data in compressed_levels {
        output.write_all(&(data.len() as u32).to_le_bytes())?;
        output.write_all(&data)?;
    }

    Ok(())
}

fn compress_dxt_level(
    texture: &Texture,
    output_format: OutputFormat,
    meaningful_alpha: bool,
) -> Vec<u8> {
    let format = output_format
        .texpresso_format()
        .expect("structured format should not be DXT-compressed");
    let mut rgba: Vec<u8> = texture.pixels.iter().flatten().copied().collect();
    if matches!(output_format, OutputFormat::Dxt3 | OutputFormat::Dxt5) && !meaningful_alpha {
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 0xFF;
        }
    }
    let mut output =
        vec![0u8; format.compressed_size(texture.width as usize, texture.height as usize)];
    format.compress(
        &rgba,
        texture.width as usize,
        texture.height as usize,
        Params {
            algorithm: Algorithm::ClusterFit,
            weigh_colour_by_alpha: output_format == OutputFormat::Dxt5,
            ..Params::default()
        },
        &mut output,
    );
    output
}

fn validate_texture(texture: &Texture) -> Result<(), AceError> {
    if texture.width == 0 || texture.height == 0 {
        return Err(AceError::EmptyTexture);
    }
    let expected_pixels = texture.width as usize * texture.height as usize;
    if texture.pixels.len() != expected_pixels {
        return Err(AceError::PixelCountMismatch);
    }
    Ok(())
}

fn build_mipmaps(base: &Texture) -> Vec<Texture> {
    let mut levels = vec![base.clone()];
    while levels.last().unwrap().width > 1 {
        levels.push(downsample_2x2(levels.last().unwrap()));
    }
    levels
}

fn downsample_2x2(source: &Texture) -> Texture {
    let width = source.width / 2;
    let height = source.height / 2;
    let mut pixels = Vec::with_capacity((width * height) as usize);

    for y in 0..height {
        for x in 0..width {
            let source_x = x * 2;
            let source_y = y * 2;
            let indexes = [
                (source_y * source.width + source_x) as usize,
                (source_y * source.width + source_x + 1) as usize,
                ((source_y + 1) * source.width + source_x) as usize,
                ((source_y + 1) * source.width + source_x + 1) as usize,
            ];
            let mut pixel = [0u8; 4];
            for channel in 0..4 {
                let sum: u16 = indexes
                    .iter()
                    .map(|&index| source.pixels[index][channel] as u16)
                    .sum();
                pixel[channel] = ((sum + 2) / 4) as u8;
            }
            pixels.push(pixel);
        }
    }

    Texture {
        width,
        height,
        pixels,
        source_has_alpha: source.source_has_alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_uncompressed_structured_rgb8_ace_for_opaque_texture() {
        let texture = Texture {
            width: 2,
            height: 1,
            pixels: vec![[0x11, 0x22, 0x33, 0xff], [0xaa, 0xbb, 0xcc, 0xff]],
            source_has_alpha: false,
        };
        let mut bytes = Vec::new();

        write_ace(&texture, &mut bytes).unwrap();

        assert_eq!(&bytes[..16], b"SIMISA@@@@@@@@@@");
        assert_eq!(&bytes[16..20], &[1, 0, 0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 3);
        assert_eq!(bytes.len(), 16 + 4 + 5 * 4 + 128 + 3 * 16 + 4 + 6);
        assert!(bytes.ends_with(&[0x11, 0xaa, 0x22, 0xbb, 0x33, 0xcc]));
    }

    #[test]
    fn writes_uncompressed_structured_rgb8_ace_when_source_alpha_is_fully_opaque() {
        let texture = Texture {
            width: 2,
            height: 1,
            pixels: vec![[0x11, 0x22, 0x33, 0xff], [0xaa, 0xbb, 0xcc, 0xff]],
            source_has_alpha: true,
        };
        let mut bytes = Vec::new();

        write_ace(&texture, &mut bytes).unwrap();

        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 3);
        assert_eq!(bytes.len(), 16 + 4 + 5 * 4 + 128 + 3 * 16 + 4 + 6);
        assert!(bytes.ends_with(&[0x11, 0xaa, 0x22, 0xbb, 0x33, 0xcc]));
    }

    #[test]
    fn writes_rgb8_ace_when_source_alpha_is_only_tiny_high_alpha_noise() {
        let mut pixels = vec![[0x11, 0x22, 0x33, 0xff]; 200];
        pixels[0][3] = 0xe0;
        let texture = Texture {
            width: 20,
            height: 10,
            pixels,
            source_has_alpha: true,
        };
        let mut bytes = Vec::new();

        write_ace(&texture, &mut bytes).unwrap();

        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 3);
    }

    #[test]
    fn writes_uncompressed_structured_rgba8_ace_when_source_alpha_is_used() {
        let texture = Texture {
            width: 2,
            height: 1,
            pixels: vec![[0x11, 0x22, 0x33, 0xff], [0xaa, 0xbb, 0xcc, 0x80]],
            source_has_alpha: true,
        };
        let mut bytes = Vec::new();

        write_ace(&texture, &mut bytes).unwrap();

        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 4);
        assert_eq!(bytes.len(), 16 + 4 + 5 * 4 + 128 + 4 * 16 + 4 + 8);
        assert!(bytes.ends_with(&[0x11, 0xaa, 0x22, 0xbb, 0x33, 0xcc, 0xff, 0x80]));
    }

    #[test]
    fn writes_raw_dxt1_ace() {
        let texture = Texture {
            width: 4,
            height: 4,
            pixels: vec![[0x11, 0x22, 0x33, 0xff]; 16],
            source_has_alpha: false,
        };
        let mut bytes = Vec::new();

        write_ace_with_options(
            &texture,
            &mut bytes,
            AceOptions {
                format: AceFormat::Dxt1,
                ..AceOptions::default()
            },
        )
        .unwrap();

        assert_eq!(&bytes[..16], b"SIMISA@@@@@@@@@@");
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0x14);
        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 0x12);
        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 3);
        let table_start = 16 + 4 + 5 * 4 + 128 + 3 * 16;
        assert_eq!(
            u32::from_le_bytes(bytes[table_start..table_start + 4].try_into().unwrap()),
            220
        );
        assert_eq!(
            u32::from_le_bytes(bytes[table_start + 4..table_start + 8].try_into().unwrap()),
            8
        );
        assert_eq!(bytes.len(), table_start + 4 + 4 + 8);
    }

    #[test]
    fn auto_dxt_chooses_dxt5_for_meaningful_alpha() {
        let texture = Texture {
            width: 4,
            height: 4,
            pixels: vec![[0x11, 0x22, 0x33, 0x80]; 16],
            source_has_alpha: true,
        };
        let mut bytes = Vec::new();

        write_ace_with_options(
            &texture,
            &mut bytes,
            AceOptions {
                format: AceFormat::AutoDxt,
                ..AceOptions::default()
            },
        )
        .unwrap();

        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 0x16);
    }

    #[test]
    fn dxt5_writes_alpha_channel_even_when_source_alpha_is_ignored() {
        let mut pixels = vec![[0x11, 0x22, 0x33, 0xff]; 256];
        pixels[0][3] = 0xe0;
        let texture = Texture {
            width: 16,
            height: 16,
            pixels,
            source_has_alpha: true,
        };
        let mut bytes = Vec::new();

        write_ace_with_options(
            &texture,
            &mut bytes,
            AceOptions {
                format: AceFormat::Dxt5,
                ..AceOptions::default()
            },
        )
        .unwrap();

        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 0x16);
        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 4);
        let table_start = 16 + 4 + 5 * 4 + 128 + 4 * 16;
        assert_eq!(
            u32::from_le_bytes(bytes[table_start..table_start + 4].try_into().unwrap()),
            236
        );
        assert_eq!(
            u32::from_le_bytes(bytes[table_start + 4..table_start + 8].try_into().unwrap()),
            256
        );
    }

    #[test]
    fn skips_dxt5_file_conversion_when_source_has_no_alpha_channel() {
        let unique = format!("png2ace_no_alpha_{}", std::process::id());
        let input = std::env::temp_dir().join(format!("{unique}.png"));
        let output = std::env::temp_dir().join(format!("{unique}.ace"));
        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&output);

        let image = image::RgbImage::from_pixel(4, 4, image::Rgb([0x11, 0x22, 0x33]));
        image.save(&input).unwrap();

        let report = convert_image_file(
            &input,
            &output,
            ConversionOptions {
                format: AceFormat::Dxt5,
                ..ConversionOptions::default()
            },
        )
        .unwrap();

        assert!(report.skipped);
        assert!(!output.exists());
        assert!(
            report
                .message
                .as_deref()
                .unwrap()
                .contains("has no alpha channel")
        );

        std::fs::remove_file(input).unwrap();
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn rejects_wrong_pixel_count() {
        let texture = Texture {
            width: 2,
            height: 2,
            pixels: vec![[0; 4]],
            source_has_alpha: false,
        };
        assert!(matches!(
            write_ace(&texture, Vec::new()),
            Err(AceError::PixelCountMismatch)
        ));
    }

    #[test]
    fn rejects_zero_dimensions() {
        let texture = Texture {
            width: 0,
            height: 1,
            pixels: Vec::new(),
            source_has_alpha: false,
        };
        assert!(matches!(
            write_ace(&texture, Vec::new()),
            Err(AceError::EmptyTexture)
        ));
    }

    #[test]
    fn writes_mipmaps_for_square_power_of_two_texture() {
        let texture = Texture {
            width: 2,
            height: 2,
            pixels: vec![
                [10, 20, 30, 0xff],
                [20, 30, 40, 0xff],
                [30, 40, 50, 0xff],
                [40, 50, 60, 0xff],
            ],
            source_has_alpha: false,
        };
        let mut bytes = Vec::new();

        write_ace_with_options(
            &texture,
            &mut bytes,
            AceOptions {
                mipmaps: true,
                ..AceOptions::default()
            },
        )
        .unwrap();

        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 1);
        let table_start = 16 + 4 + 5 * 4 + 128 + 3 * 16;
        assert_eq!(bytes.len(), table_start + (2 + 1) * 4 + (2 * 2 * 3) + 3);
        assert!(bytes.ends_with(&[25, 35, 45]));
    }

    #[test]
    fn rejects_mipmaps_for_non_square_non_power_of_two_texture() {
        let texture = Texture {
            width: 3,
            height: 2,
            pixels: vec![[0; 4]; 6],
            source_has_alpha: false,
        };
        assert!(matches!(
            write_ace_with_options(
                &texture,
                Vec::new(),
                AceOptions {
                    mipmaps: true,
                    ..AceOptions::default()
                }
            ),
            Err(AceError::InvalidMipmaps)
        ));
    }
}
