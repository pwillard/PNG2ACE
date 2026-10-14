#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, TextStyle};
use png2ace::{
    AceFormat, ConversionOptions, collect_image_files, convert_image_file,
    convert_image_folder_with_progress,
};
use rfd::FileDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    SingleFile,
    Folder,
}

struct Png2AceGui {
    mode: InputMode,
    source: String,
    output: String,
    mipmaps: bool,
    format: AceFormat,
    log: Vec<String>,
}

impl Default for Png2AceGui {
    fn default() -> Self {
        Self {
            mode: InputMode::Folder,
            source: ".".to_owned(),
            output: String::new(),
            mipmaps: false,
            format: AceFormat::Structured,
            log: vec!["Ready.".to_owned()],
        }
    }
}

impl eframe::App for Png2AceGui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading(
            egui::RichText::new("PNG/TGA to ACE Converter")
                .color(Color32::WHITE)
                .strong(),
        );
        ui.label(
            egui::RichText::new("Convert PNG/TGA textures to Microsoft Train Simulator ACE files.")
                .color(ACCENT),
        );
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Input:");
            ui.radio_value(&mut self.mode, InputMode::SingleFile, "Single image");
            ui.radio_value(&mut self.mode, InputMode::Folder, "Folder batch");
        });

        ui.horizontal(|ui| {
            ui.label("Source:");
            ui.text_edit_singleline(&mut self.source);
            if ui.button("Browse...").clicked() {
                match self.mode {
                    InputMode::SingleFile => {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Supported images", &["png", "tga", "targa"])
                            .add_filter("PNG images", &["png"])
                            .add_filter("TGA images", &["tga", "targa"])
                            .pick_file()
                        {
                            self.source = path.display().to_string();
                            if self.output.is_empty() {
                                self.output = path.with_extension("ace").display().to_string();
                            }
                        }
                    }
                    InputMode::Folder => {
                        if let Some(path) = FileDialog::new().pick_folder() {
                            self.source = path.display().to_string();
                        }
                    }
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Output:");
            ui.text_edit_singleline(&mut self.output);
            if ui.button("Browse...").clicked() {
                match self.mode {
                    InputMode::SingleFile => {
                        let mut dialog = FileDialog::new().add_filter("ACE textures", &["ace"]);
                        if !self.source.trim().is_empty() {
                            let default_name =
                                PathBuf::from(self.source.trim()).with_extension("ace");
                            if let Some(name) = default_name.file_name() {
                                dialog = dialog.set_file_name(name.to_string_lossy());
                            }
                        }
                        if let Some(path) = dialog.save_file() {
                            self.output = path.display().to_string();
                        }
                    }
                    InputMode::Folder => {
                        if let Some(path) = FileDialog::new().pick_folder() {
                            self.output = path.display().to_string();
                        }
                    }
                }
            }
        });

        ui.checkbox(
            &mut self.mipmaps,
            "Generate mipmaps (square power-of-two PNGs only)",
        );

        ui.horizontal(|ui| {
            ui.label("ACE format:");
            egui::ComboBox::from_id_salt("ace_format")
                .selected_text(format_label(self.format))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.format,
                        AceFormat::Structured,
                        format_label(AceFormat::Structured),
                    );
                    ui.selectable_value(
                        &mut self.format,
                        AceFormat::Dxt1,
                        format_label(AceFormat::Dxt1),
                    );
                    ui.selectable_value(
                        &mut self.format,
                        AceFormat::Dxt3,
                        format_label(AceFormat::Dxt3),
                    );
                    ui.selectable_value(
                        &mut self.format,
                        AceFormat::Dxt5,
                        format_label(AceFormat::Dxt5),
                    );
                    ui.selectable_value(
                        &mut self.format,
                        AceFormat::AutoDxt,
                        format_label(AceFormat::AutoDxt),
                    );
                });
        });

        ui.horizontal(|ui| {
            if ui.button("Convert").clicked() {
                self.run_conversion();
            }
            if ui.button("Clear log").clicked() {
                self.log.clear();
            }
        });

        ui.separator();
        ui.label("Status:");
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.log {
                    ui.label(line);
                }
            });
    }
}

impl Png2AceGui {
    fn run_conversion(&mut self) {
        self.log.clear();
        match self.mode {
            InputMode::SingleFile => self.convert_single_file(),
            InputMode::Folder => self.convert_folder(),
        }
    }

    fn convert_single_file(&mut self) {
        let source = self.source.trim();
        if source.is_empty() {
            self.log.push("Choose a source PNG/TGA first.".to_owned());
            return;
        }

        let input = PathBuf::from(source);
        let output = if self.output.trim().is_empty() {
            input.with_extension("ace")
        } else {
            PathBuf::from(self.output.trim())
        };

        self.log.push(format!(
            "Converting {} -> {} ...",
            input.display(),
            output.display()
        ));
        match convert_image_file(
            &input,
            &output,
            ConversionOptions {
                mipmaps: self.mipmaps,
                format: self.format,
                ..ConversionOptions::default()
            },
        ) {
            Ok(report) => self.log.push(report_log_line(&report)),
            Err(error) => self.log.push(format!("ERROR: {error}")),
        }
    }

    fn convert_folder(&mut self) {
        let input = if self.source.trim().is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(self.source.trim())
        };
        let output = if self.output.trim().is_empty() {
            None
        } else {
            Some(PathBuf::from(self.output.trim()))
        };

        let image_count = match collect_image_files(&input) {
            Ok(images) => images.len(),
            Err(error) => {
                self.log.push(format!("ERROR: {error}"));
                return;
            }
        };
        if image_count == 0 {
            self.log
                .push(format!("No PNG/TGA files found in {}", input.display()));
            return;
        }

        self.log.push(format!(
            "Found {image_count} PNG/TGA file(s) in {}",
            input.display()
        ));

        let options = ConversionOptions {
            mipmaps: self.mipmaps,
            format: self.format,
            ..ConversionOptions::default()
        };
        let result = convert_image_folder_with_progress(
            &input,
            output.as_ref(),
            options,
            |index, total, png, ace| {
                self.log.push(format!(
                    "[{index}/{total}] {} -> {}",
                    png.display(),
                    ace.display()
                ));
            },
        );

        match result {
            Ok(reports) => {
                for report in &reports {
                    self.log.push(report_log_line(report));
                }
                self.log
                    .push(format!("Converted {} PNG/TGA file(s).", reports.len()));
            }
            Err(error) => self.log.push(format!("ERROR: {error}")),
        }
    }
}

fn format_label(format: AceFormat) -> &'static str {
    match format {
        AceFormat::Structured => "Structured RGB/RGBA",
        AceFormat::Dxt1 => "DXT1 / BC1",
        AceFormat::Dxt3 => "DXT3 / BC2 (for alpha images)",
        AceFormat::Dxt5 => "DXT5 / BC3 (for alpha images)",
        AceFormat::AutoDxt => "Auto DXT",
    }
}

fn report_log_line(report: &png2ace::ConversionReport) -> String {
    if report.skipped {
        format!(
            "Skipped: {}",
            report.message.as_deref().unwrap_or("format not applicable")
        )
    } else {
        format!(
            "Done: {}x{}, {} channel {} ACE written to {}",
            report.width,
            report.height,
            report.channel_count,
            report.format,
            report.output.display()
        )
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([760.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PNG/TGA to ACE Converter",
        options,
        Box::new(|cc| {
            apply_shape_packer_style(&cc.egui_ctx);
            Ok(Box::new(Png2AceGui::default()))
        }),
    )
}

const ACCENT: Color32 = Color32::from_rgb(245, 176, 69);

fn apply_shape_packer_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(Color32::WHITE);
    visuals.weak_text_color = Some(Color32::from_rgb(210, 220, 230));
    visuals.panel_fill = Color32::from_rgb(43, 43, 43);
    visuals.window_fill = Color32::from_rgb(43, 43, 43);
    visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(120, 120, 120));
    visuals.extreme_bg_color = Color32::from_rgb(16, 16, 16);
    visuals.text_edit_bg_color = Some(Color32::from_rgb(20, 20, 20));
    visuals.faint_bg_color = Color32::from_rgb(34, 34, 34);
    visuals.code_bg_color = Color32::from_rgb(18, 18, 18);
    visuals.selection.bg_fill = Color32::from_rgb(168, 108, 22);
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = ACCENT;
    visuals.error_fg_color = Color32::from_rgb(255, 96, 96);

    let border = Stroke::new(1.0, Color32::from_rgb(150, 150, 150));
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(32, 32, 32);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(32, 32, 32);
    visuals.widgets.noninteractive.bg_stroke = border;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(55, 55, 55);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(58, 58, 58);
    visuals.widgets.inactive.bg_stroke = border;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(76, 76, 76);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(82, 82, 82);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::BLACK);

    visuals.widgets.open.bg_fill = Color32::from_rgb(64, 64, 64);
    visuals.widgets.open.weak_bg_fill = Color32::from_rgb(64, 64, 64);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(24.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );
    ctx.set_style_of(egui::Theme::Dark, style);
}
