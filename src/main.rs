use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use png2ace::{
    AceFormat, ConversionOptions, collect_image_files, convert_image_file, image_output_path,
    validate_unique_outputs,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Structured,
    Dxt1,
    Dxt3,
    Dxt5,
    AutoDxt,
}

impl From<FormatArg> for AceFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Structured => AceFormat::Structured,
            FormatArg::Dxt1 => AceFormat::Dxt1,
            FormatArg::Dxt3 => AceFormat::Dxt3,
            FormatArg::Dxt5 => AceFormat::Dxt5,
            FormatArg::AutoDxt => AceFormat::AutoDxt,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "png2ace",
    version,
    about = "Convert PNG/TGA images to MSTS ACE textures"
)]
struct Args {
    /// Source PNG/TGA image or folder. Defaults to the current folder.
    input: Option<PathBuf>,
    /// Destination ACE file for one image, or destination folder for folder conversion.
    output: Option<PathBuf>,
    /// Generate a complete mipmap chain. Requires square power-of-two dimensions.
    #[arg(long)]
    mipmaps: bool,
    /// ACE output format.
    #[arg(long, value_enum, default_value_t = FormatArg::Structured)]
    format: FormatArg,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let input = args.input.unwrap_or_else(|| PathBuf::from("."));
    let format = AceFormat::from(args.format);

    if input.is_dir() {
        convert_folder(&input, args.output.as_deref(), args.mipmaps, format)?;
    } else {
        let output = args.output.unwrap_or_else(|| input.with_extension("ace"));
        convert_image(&input, &output, args.mipmaps, format, None)?;
    }

    Ok(())
}

fn convert_folder(
    input_folder: &Path,
    output_folder: Option<&Path>,
    mipmaps: bool,
    format: AceFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let images = collect_image_files(input_folder)?;

    if images.is_empty() {
        println!("no PNG/TGA files found in {}", input_folder.display());
        return Ok(());
    }

    println!(
        "found {} PNG/TGA file(s) in {}",
        images.len(),
        input_folder.display()
    );

    let output_folder = output_folder.unwrap_or(input_folder);
    fs::create_dir_all(output_folder)?;
    validate_unique_outputs(output_folder, &images)?;
    for (index, image) in images.iter().enumerate() {
        let output = image_output_path(output_folder, image)?;
        print!(
            "[{}/{}] converting {} -> {} ... ",
            index + 1,
            images.len(),
            image.display(),
            output.display()
        );
        io::stdout().flush()?;
        let report = convert_image_file(
            image,
            output,
            ConversionOptions {
                mipmaps,
                format,
                ..ConversionOptions::default()
            },
        )?;
        print_report(&report, mipmaps);
    }

    println!("converted {} PNG/TGA file(s)", images.len());
    Ok(())
}

fn convert_image(
    input: &Path,
    output: &Path,
    mipmaps: bool,
    format: AceFormat,
    batch_position: Option<(usize, usize)>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some((index, total)) = batch_position {
        print!("[{index}/{total}] ");
    }
    print!(
        "converting {} -> {} ... ",
        input.display(),
        output.display()
    );
    io::stdout().flush()?;

    let report = convert_image_file(
        input,
        output,
        ConversionOptions {
            mipmaps,
            format,
            ..ConversionOptions::default()
        },
    )?;
    print_report(&report, mipmaps);
    Ok(())
}

fn print_report(report: &png2ace::ConversionReport, mipmaps: bool) {
    if report.skipped {
        println!(
            "skipped ({})",
            report.message.as_deref().unwrap_or("format not applicable")
        );
    } else {
        println!(
            "done ({}x{}, {} channel {}, {})",
            report.width,
            report.height,
            report.channel_count,
            report.format,
            if mipmaps {
                "with mipmaps"
            } else {
                "no mipmaps"
            }
        );
    }
}
