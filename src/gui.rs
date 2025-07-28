#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use webpify::{
    ConversionOptions, ConversionReport, CompressionMode, WebpifyCore, ProgressReporter, ReplaceInputMode, ReportFormat,
};

/// Icon definitions optimized for Windows 11 with semantic meaning
struct Icons;

impl Icons {
    // Use semantic icons with consistent spacing and Windows 11 compatibility
    // These are carefully chosen Unicode symbols that render consistently
    
    // Tab icons - semantic and meaningful
    const FOLDER: &'static str = "📁";     // Folder for input/output
    const SETTINGS: &'static str = "⚙️";    // Gear for settings
    const ADVANCED: &'static str = "🔧";    // Wrench for advanced options
    const PROGRESS: &'static str = "📊";    // Chart for progress
    const RESULTS: &'static str = "📈";     // Chart for results
    
    // Action icons - clear and intuitive
    const START: &'static str = "▶️";       // Play button for start
    const STOP: &'static str = "⏹️";       // Stop button
    const CLEAR: &'static str = "🗑️";      // Trash for clear
    
    // Status icons - universally understood
    const WARNING: &'static str = "⚠️";     // Warning triangle
    const INFO: &'static str = "ℹ️";        // Information
    
    // Helper method to create properly spaced icon text for Windows 11
    fn with_text(icon: &str, text: &str) -> String {
        // Use non-breaking space for consistent spacing on Windows 11
        format!("{}\u{00A0}{}", icon, text)
    }
}

/// Additional helper for creating consistent UI elements
struct UiHelpers;

impl UiHelpers {
    /// Create a status indicator with appropriate color
    fn status_indicator(ui: &mut egui::Ui, icon: &str, text: &str, status: StatusType) {
        let color = match status {
            StatusType::Warning => egui::Color32::ORANGE,
            StatusType::Info => egui::Color32::GRAY,
        };
        ui.colored_label(color, format!("{} {}", icon, text));
    }
}

#[derive(Debug)]
enum StatusType {
    Warning,
    Info,
}

/// Information about a file to be converted (for preview)
#[derive(Debug, Clone)]
struct PreviewFileInfo {
    path: PathBuf,
    size: u64,
    format: String,
    estimated_output_size: Option<u64>,
}

/// Main GUI application structure
pub struct WebpifyGuiApp {
    // UI State
    current_tab: Tab,
    is_converting: bool,
    progress: f32,
    total_files: usize,
    processed_files: usize,
    failed_files: usize,
    
    // Modal dialogs
    show_preview_window: bool,
    show_help_window: bool,
    preview_files: Vec<PreviewFileInfo>,
    
    // Input/Output Configuration
    input_dir: String,
    output_dir: String,
    output_dir_auto: bool,
    
    // Basic Conversion Settings
    quality: u8,
    mode: CompressionMode,
    threads: String,
    threads_auto: bool,
    
    // File Processing Settings
    formats: String,
    overwrite: bool,
    preserve_structure: bool,
    max_size: String,
    min_size: u64,
    prescan: bool,
    reencode_webp: bool,
    
    // Advanced Settings
    replace_input: ReplaceInputMode,
    dry_run: bool,
    verbose: bool,
    quiet: bool,
    
    // Report Settings
    generate_report: bool,
    report_format: ReportFormat,
    
    // Configuration Management
    config_file: String,
    profile: String,
    
    // Results
    last_report: Option<ConversionReport>,
    error_message: Option<String>,
    conversion_log: Vec<String>,
    
    // Progress reporting
    progress_reporter: Arc<Mutex<GuiProgressReporter>>,
}

#[derive(Debug, PartialEq)]
enum Tab {
    Input,
    Settings,
    Advanced,
    Progress,
    Results,
}

impl Default for WebpifyGuiApp {
    fn default() -> Self {
        Self {
            // UI State
            current_tab: Tab::Input,
            is_converting: false,
            progress: 0.0,
            total_files: 0,
            processed_files: 0,
            failed_files: 0,
            
            // Modal dialogs
            show_preview_window: false,
            show_help_window: false,
            preview_files: Vec::new(),
            
            // Input/Output Configuration
            input_dir: String::new(),
            output_dir: String::new(),
            output_dir_auto: true,
            
            // Basic Conversion Settings
            quality: 80,
            mode: CompressionMode::Lossless,
            threads: num_cpus::get().to_string(),
            threads_auto: true,
            
            // File Processing Settings
            formats: "jpg,jpeg,png,gif,bmp,tiff,webp".to_string(),
            overwrite: false,
            preserve_structure: true,
            max_size: String::new(),
            min_size: 1,
            prescan: true,
            reencode_webp: false,
            
            // Advanced Settings
            replace_input: ReplaceInputMode::Off,
            dry_run: false,
            verbose: false,
            quiet: false,
            
            // Report Settings
            generate_report: false,
            report_format: ReportFormat::Json,
            
            // Configuration Management
            config_file: String::new(),
            profile: String::new(),
            
            // Results
            last_report: None,
            error_message: None,
            conversion_log: Vec::new(),
            
            // Progress reporting
            progress_reporter: Arc::new(Mutex::new(GuiProgressReporter::new())),
        }
    }
}

impl eframe::App for WebpifyGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update progress from background thread
        if let Ok(reporter) = self.progress_reporter.lock() {
            self.total_files = reporter.total_files;
            self.processed_files = reporter.processed_files;
            self.failed_files = reporter.failed_files;
            
            if self.total_files > 0 {
                self.progress = self.processed_files as f32 / self.total_files as f32;
            }
            
            if reporter.finished {
                self.is_converting = false;
                if let Some(report) = &reporter.report {
                    self.last_report = Some(report.clone());
                    // Auto-switch to results tab when conversion finishes
                    self.current_tab = Tab::Results;
                }
                if let Some(error) = &reporter.error {
                    self.error_message = Some(error.clone());
                }
            }
            
            // Collect conversion logs
            for log in &reporter.logs {
                if !self.conversion_log.contains(log) {
                    self.conversion_log.push(log.clone());
                }
            }
        }

        // Enhanced top panel with step indicator
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                // Main header
                ui.horizontal(|ui| {
                    ui.heading("Webpify - Batch WebP Converter");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.is_converting {
                            ui.spinner();
                            ui.label("Converting...");
                        }
                    });
                });
                
                ui.separator();
                
                // Step indicator with enhanced visual flow
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    
                    // Step 1: Input
                    let step1_color = if matches!(self.current_tab, Tab::Input) { 
                        egui::Color32::BLUE 
                    } else if !self.input_dir.is_empty() { 
                        egui::Color32::GREEN 
                    } else { 
                        egui::Color32::GRAY 
                    };
                    ui.colored_label(step1_color, "1️⃣ Input");
                    
                    ui.label("→");
                    
                    // Step 2: Settings
                    let step2_color = if matches!(self.current_tab, Tab::Settings) { 
                        egui::Color32::BLUE 
                    } else { 
                        egui::Color32::GRAY 
                    };
                    ui.colored_label(step2_color, "2️⃣ Settings");
                    
                    ui.label("→");
                    
                    // Step 3: Advanced (Optional)
                    let step3_color = if matches!(self.current_tab, Tab::Advanced) { 
                        egui::Color32::BLUE 
                    } else { 
                        egui::Color32::LIGHT_GRAY 
                    };
                    ui.colored_label(step3_color, "3️⃣ Advanced");
                    
                    ui.label("→");
                    
                    // Step 4: Convert
                    let step4_color = if matches!(self.current_tab, Tab::Progress) || self.is_converting { 
                        egui::Color32::BLUE 
                    } else { 
                        egui::Color32::GRAY 
                    };
                    ui.colored_label(step4_color, "4️⃣ Convert");
                    
                    ui.label("→");
                    
                    // Step 5: Results
                    let step5_color = if matches!(self.current_tab, Tab::Results) || self.last_report.is_some() { 
                        egui::Color32::BLUE 
                    } else { 
                        egui::Color32::GRAY 
                    };
                    ui.colored_label(step5_color, "5️⃣ Results");
                });
            });
        });

        // Enhanced tab navigation with better visual design
        egui::TopBottomPanel::top("tab_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 200);
                
                // Use consistent spacing for all tabs with enhanced styling
                if ui.selectable_value(&mut self.current_tab, Tab::Input, &Icons::with_text(Icons::FOLDER, "Input & Output")).clicked() {
                    // Auto-validate when switching to input tab
                }
                
                ui.selectable_value(&mut self.current_tab, Tab::Settings, &Icons::with_text(Icons::SETTINGS, "Settings"));
                ui.selectable_value(&mut self.current_tab, Tab::Advanced, &Icons::with_text(Icons::ADVANCED, "Advanced"));
                
                if self.is_converting || self.total_files > 0 {
                    ui.selectable_value(&mut self.current_tab, Tab::Progress, &Icons::with_text(Icons::PROGRESS, "Progress"));
                }
                if self.last_report.is_some() || self.error_message.is_some() {
                    ui.selectable_value(&mut self.current_tab, Tab::Results, &Icons::with_text(Icons::RESULTS, "Results"));
                }
                
                // Quick actions on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("💡 Help").clicked() {
                        self.show_help_window = true;
                    }
                    
                    if !self.input_dir.is_empty() && !self.is_converting {
                        if ui.small_button("🔍 Preview").clicked() {
                            // Clear any previous error messages
                            self.error_message = None;
                            self.show_preview_window = true;
                            self.generate_preview();
                        }
                    }
                });
            });
        });

        // Enhanced bottom panel with better action layout
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.separator();
            
            // Summary info bar
            ui.horizontal(|ui| {
                if !self.input_dir.is_empty() {
                    ui.label(format!("📁 Input: {}", 
                        std::path::Path::new(&self.input_dir)
                            .file_name()
                            .map(|n| n.to_string_lossy())
                            .unwrap_or("Unknown".into())
                    ));
                    
                    if !self.output_dir.is_empty() {
                        ui.separator();
                        ui.label(format!("📂 Output: {}", 
                            std::path::Path::new(&self.output_dir)
                                .file_name()
                                .map(|n| n.to_string_lossy())
                                .unwrap_or("Auto".into())
                        ));
                    }
                }
            });
            
            ui.separator();
            
            // Action buttons with better layout
            ui.horizontal(|ui| {
                let can_convert = !self.input_dir.is_empty() && !self.is_converting;
                
                // Primary action button with enhanced styling
                let start_btn = ui.add_sized([140.0, 36.0], 
                    egui::Button::new(&Icons::with_text(Icons::START, "Start Conversion"))
                        .fill(if can_convert { 
                            egui::Color32::from_rgb(0, 150, 50) 
                        } else { 
                            egui::Color32::GRAY 
                        })
                ).on_hover_text("Begin converting images to WebP format");
                
                if start_btn.clicked() && can_convert {
                    self.start_conversion();
                }

                // Secondary action buttons with improved spacing
                ui.add_space(10.0);
                
                let stop_btn = self.secondary_button(ui, &Icons::with_text(Icons::STOP, "Stop"))
                    .on_hover_text("Stop the current conversion process");
                    
                if stop_btn.clicked() && self.is_converting {
                    self.is_converting = false;
                }

                let clear_btn = self.secondary_button(ui, &Icons::with_text(Icons::CLEAR, "Clear"))
                    .on_hover_text("Clear all results and reset progress");
                    
                if clear_btn.clicked() {
                    self.clear_results();
                }

                // Status and validation on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !can_convert && !self.input_dir.is_empty() {
                        UiHelpers::status_indicator(ui, Icons::WARNING, "Conversion in progress", StatusType::Warning);
                    } else if self.input_dir.is_empty() {
                        UiHelpers::status_indicator(ui, Icons::INFO, "Select an input directory to begin", StatusType::Info);
                    } else if can_convert {
                        // Show estimated info
                        ui.label("✅ Ready to convert");
                    }
                });
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.current_tab {
                    Tab::Input => self.show_input_tab(ui),
                    Tab::Settings => self.show_settings_tab(ui),
                    Tab::Advanced => self.show_advanced_tab(ui),
                    Tab::Progress => self.show_progress_tab(ui),
                    Tab::Results => self.show_results_tab(ui),
                }
            });
        });

        // Request repaint if converting
        if self.is_converting {
            ctx.request_repaint();
        }

        // Show modal windows
        self.show_preview_modal(ctx);
        self.show_help_modal(ctx);
    }
}

impl WebpifyGuiApp {
    /// Create a secondary action button
    fn secondary_button(&self, ui: &mut egui::Ui, text: &str) -> egui::Response {
        ui.add_sized([100.0, 32.0], egui::Button::new(text))
    }

    fn show_input_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(&Icons::with_text(Icons::FOLDER, "Input & Output Configuration"));
        ui.add_space(10.0);

        // Enhanced layout with cards and better visual hierarchy
        ui.horizontal(|ui| {
            // Left column - Input Configuration
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width() * 0.48);
                
                // Input Directory Card
                ui.group(|ui| {
                    ui.set_min_width(300.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📁 Input Directory").size(16.0).strong());
                            if !self.input_dir.is_empty() {
                                let path = PathBuf::from(&self.input_dir);
                                if path.exists() {
                                    ui.label(egui::RichText::new("✅").color(egui::Color32::GREEN));
                                } else {
                                    ui.label(egui::RichText::new("❌").color(egui::Color32::RED));
                                }
                            }
                        });
                        
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            ui.add_sized([280.0, 24.0], egui::TextEdit::singleline(&mut self.input_dir)
                                .hint_text("Select the folder containing images to convert"));
                            
                            if ui.button("📂 Browse").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title("Select Input Directory")
                                    .pick_folder() {
                                    self.input_dir = path.display().to_string();
                                    
                                    // Auto-set output directory if enabled
                                    if self.output_dir_auto {
                                        let mut output_path = path;
                                        output_path.push("webp_output");
                                        self.output_dir = output_path.display().to_string();
                                    }
                                }
                            }
                        });
                        
                        if !self.input_dir.is_empty() {
                            ui.add_space(5.0);
                            let path = PathBuf::from(&self.input_dir);
                            if path.exists() {
                                ui.label(egui::RichText::new("✅ Directory exists and is accessible")
                                    .color(egui::Color32::DARK_GREEN).size(12.0));
                            } else {
                                ui.label(egui::RichText::new("❌ Directory does not exist or is not accessible")
                                    .color(egui::Color32::DARK_RED).size(12.0));
                            }
                        } else {
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("ℹ️ Choose a folder containing images to convert")
                                .color(egui::Color32::GRAY).size(12.0));
                        }
                    });
                });

                ui.add_space(15.0);

                // File Format Configuration
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🎯 File Format Filter").size(16.0).strong());
                        ui.add_space(10.0);
                        
                        ui.label("Supported input formats (comma-separated):");
                        ui.add(egui::TextEdit::multiline(&mut self.formats)
                            .desired_rows(2)
                            .hint_text("jpg,jpeg,png,gif,bmp,tiff,webp"));
                        
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.small_button("📷 Photos").clicked() {
                                self.formats = "jpg,jpeg,png".to_string();
                            }
                            if ui.small_button("🖼️ Common").clicked() {
                                self.formats = "jpg,jpeg,png,gif,bmp,tiff".to_string();
                            }
                            if ui.small_button("🌐 All").clicked() {
                                self.formats = "jpg,jpeg,png,gif,bmp,tiff,webp".to_string();
                            }
                        });
                    });
                });
            });

            ui.add_space(10.0);

            // Right column - Output Configuration
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width());
                
                // Output Directory
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📂 Output Directory").size(16.0).strong());
                        ui.add_space(10.0);
                        
                        ui.checkbox(&mut self.output_dir_auto, "🤖 Auto-generate output directory");
                        
                        ui.add_space(8.0);
                        
                        ui.horizontal(|ui| {
                            ui.add_enabled(!self.output_dir_auto, 
                                egui::TextEdit::singleline(&mut self.output_dir)
                                    .desired_width(280.0)
                                    .hint_text("Leave empty to use default"));
                            
                            if ui.add_enabled(!self.output_dir_auto, egui::Button::new("📂 Browse")).clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title("Select Output Directory")
                                    .pick_folder() {
                                    self.output_dir = path.display().to_string();
                                }
                            }
                        });
                        
                        ui.add_space(5.0);
                        
                        // Show output path preview
                        if !self.output_dir.is_empty() {
                            ui.label(egui::RichText::new(format!("📁 Output: {}", self.output_dir))
                                .color(egui::Color32::DARK_BLUE).size(12.0));
                        } else if !self.input_dir.is_empty() {
                            let mut default_output = PathBuf::from(&self.input_dir);
                            default_output.push("webp_output");
                            ui.label(egui::RichText::new(format!("📁 Default: {}", default_output.display()))
                                .color(egui::Color32::GRAY).size(12.0));
                        }
                    });
                });

                ui.add_space(15.0);

                // Quick Setup
                ui.add_space(10.0);
                ui.collapsing("⚡ Quick Setup", |ui| {
                    if !self.input_dir.is_empty() {
                        let path = PathBuf::from(&self.input_dir);
                        if path.exists() {
                            ui.label("✅ Ready to proceed to Settings");
                            ui.add_space(5.0);
                            if ui.button("➡️ Go to Settings").clicked() {
                                self.current_tab = Tab::Settings;
                            }
                        } else {
                            ui.label("❌ Please select a valid input directory first");
                        }
                    } else {
                        ui.label("📝 Next steps:");
                        ui.label("1. Select an input directory");
                        ui.label("2. Configure output location");
                        ui.label("3. Choose file formats");
                    }
                });
            });
        });
    }

    fn show_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(&Icons::with_text(Icons::SETTINGS, "Conversion Settings"));
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            // Left column - Quality & Compression
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width() * 0.48);
                
                // Quality Presets
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🎯 Quality Presets").size(16.0).strong());
                        ui.add_space(10.0);
                        
                        // Preset buttons with hover effects
                        ui.horizontal(|ui| {
                            if ui.selectable_label(self.quality == 95, "🏆 Best").on_hover_text("95% quality, perfect for archival").clicked() {
                                self.quality = 95;
                                self.mode = CompressionMode::Lossless;
                            }
                            if ui.selectable_label(self.quality == 85, "⚖️ Balanced").on_hover_text("85% quality, good compromise").clicked() {
                                self.quality = 85;
                                self.mode = CompressionMode::Auto;
                            }
                            if ui.selectable_label(self.quality == 70, "📦 Small").on_hover_text("70% quality, smaller files").clicked() {
                                self.quality = 70;
                                self.mode = CompressionMode::Lossy;
                            }
                            if ui.selectable_label(self.quality == 50, "🗜️ Tiny").on_hover_text("50% quality, very small files").clicked() {
                                self.quality = 50;
                                self.mode = CompressionMode::Lossy;
                            }
                        });
                        
                        ui.add_space(15.0);
                        
                        // Custom quality slider
                        ui.label("Custom Quality:");
                        ui.horizontal(|ui| {
                            if ui.add(egui::Slider::new(&mut self.quality, 1..=100)
                                .suffix("%")
                                .text("Quality")).changed() {
                                // Auto-update mode based on quality
                                self.mode = match self.quality {
                                    90..=100 => CompressionMode::Lossless,
                                    60..=89 => CompressionMode::Auto,
                                    _ => CompressionMode::Lossy,
                                };
                            }
                            
                            // Quality indicator with dynamic colors
                            let (quality_text, quality_color) = match self.quality {
                                90..=100 => ("🏆 Excellent", egui::Color32::GREEN),
                                75..=89 => ("⚖️ Good", egui::Color32::BLUE),
                                50..=74 => ("📦 Fair", egui::Color32::ORANGE), 
                                _ => ("🗜️ Small", egui::Color32::RED)
                            };
                            ui.colored_label(quality_color, quality_text);
                        });
                        
                        ui.add_space(10.0);
                        
                        // Compression mode with visual feedback
                        ui.label("Compression Mode:");
                        egui::ComboBox::from_id_salt("compression_mode")
                            .selected_text(match self.mode {
                                CompressionMode::Lossless => "🏆 Lossless (Perfect Quality)",
                                CompressionMode::Lossy => "📦 Lossy (Smaller Size)",
                                CompressionMode::Auto => "🤖 Auto (Smart Choice)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.mode, CompressionMode::Lossless, "🏆 Lossless (Perfect Quality)");
                                ui.selectable_value(&mut self.mode, CompressionMode::Lossy, "📦 Lossy (Smaller Size)");
                                ui.selectable_value(&mut self.mode, CompressionMode::Auto, "🤖 Auto (Smart Choice)");
                            });
                            
                        // Mode explanation with better styling
                        ui.add_space(5.0);
                        let mode_desc = match self.mode {
                            CompressionMode::Lossless => "Perfect quality, larger files",
                            CompressionMode::Lossy => "Good quality, smaller files",
                            CompressionMode::Auto => "Automatically chooses best mode per image",
                        };
                        ui.label(egui::RichText::new(mode_desc).size(12.0).italics().color(egui::Color32::GRAY));
                    });
                });

                ui.add_space(15.0);

                // File Processing
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📁 File Processing").size(16.0).strong());
                        ui.add_space(10.0);
                        
                        ui.checkbox(&mut self.overwrite, "🔄 Overwrite existing WebP files");
                        ui.checkbox(&mut self.preserve_structure, "🗂️ Preserve directory structure");
                        ui.checkbox(&mut self.reencode_webp, "🔄 Re-encode existing WebP files");
                        
                        ui.add_space(10.0);
                        
                        // Size constraints with validation
                        ui.horizontal(|ui| {
                            ui.label("📏 Min size (KB):");
                            if ui.add(egui::DragValue::new(&mut self.min_size).range(1..=10000)).changed() {
                                // Visual feedback when changed
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("📐 Max size (MB):");
                            let _text_edit = ui.add(egui::TextEdit::singleline(&mut self.max_size)
                                .desired_width(80.0)
                                .hint_text("No limit"));
                            
                            // Validation feedback
                            if !self.max_size.is_empty() && self.max_size.parse::<u64>().is_err() {
                                ui.colored_label(egui::Color32::RED, "⚠️");
                            }
                        });
                    });
                });
            });

            ui.add_space(10.0);

            // Right column - Performance & Validation
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width());
                
                // Performance Settings
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("⚡ Performance").size(16.0).strong());
                        ui.add_space(10.0);
                        
                        ui.checkbox(&mut self.threads_auto, "🤖 Auto-detect optimal thread count");
                        
                        ui.add_space(8.0);
                        
                        ui.horizontal(|ui| {
                            ui.add_enabled(!self.threads_auto, egui::Label::new("🧵 Thread Count:"));
                            let _thread_edit = ui.add_enabled(!self.threads_auto, 
                                egui::TextEdit::singleline(&mut self.threads)
                                    .desired_width(60.0));
                            
                            if self.threads_auto {
                                ui.label(format!("(Auto: {} threads)", num_cpus::get()));
                                self.threads = num_cpus::get().to_string();
                            } else if !self.threads_auto && self.threads.parse::<usize>().is_err() {
                                ui.colored_label(egui::Color32::RED, "⚠️ Invalid");
                            }
                        });
                        
                        ui.add_space(10.0);
                        ui.checkbox(&mut self.prescan, "🔍 Enable pre-processing scan (recommended)");
                        
                        // Performance tips with collapsible section
                        ui.add_space(10.0);
                        ui.collapsing("💡 Performance Tips", |ui| {
                            ui.label(egui::RichText::new("• More threads = faster conversion").size(11.0).color(egui::Color32::GRAY));
                            ui.label(egui::RichText::new("• Pre-scan helps estimate time").size(11.0).color(egui::Color32::GRAY));
                            ui.label(egui::RichText::new("• SSD storage improves speed").size(11.0).color(egui::Color32::GRAY));
                        });
                    });
                });

                ui.add_space(15.0);

                // Validation & Preview
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("✅ Settings Summary").size(16.0).strong());
                        ui.add_space(10.0);
                        
                        ui.label(format!("🎯 Quality: {}% ({:?})", self.quality, self.mode));
                        ui.label(format!("🧵 Threads: {}", if self.threads_auto { "Auto".to_string() } else { self.threads.clone() }));
                        ui.label(format!("📏 File size: {} KB - {}", 
                            self.min_size, 
                            if self.max_size.is_empty() { "No limit".to_string() } else { format!("{} MB", self.max_size) }
                        ));
                        
                        ui.add_space(10.0);
                        
                        if !self.input_dir.is_empty() {
                            if ui.button("➡️ Continue to Advanced").clicked() {
                                self.current_tab = Tab::Advanced;
                            }
                            ui.horizontal(|ui| {
                                ui.label("or");
                                if ui.button("🚀 Start Converting Now").clicked() {
                                    self.start_conversion();
                                }
                            });
                        } else {
                            ui.label("❌ Please select input directory first");
                            if ui.button("⬅️ Back to Input").clicked() {
                                self.current_tab = Tab::Input;
                            }
                        }
                    });
                });
            });
        });
    }

    fn show_advanced_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(&Icons::with_text(Icons::ADVANCED, "Advanced Options"));
        ui.add_space(10.0);

        // Input File Handling
        ui.group(|ui| {
            ui.label("🗂️ Input File Handling");
            ui.add_space(5.0);
            
            ui.label("What to do with original files after successful conversion:");
            egui::ComboBox::from_id_salt("replace_input")
                .selected_text(match self.replace_input {
                    ReplaceInputMode::Off => "Keep original files (safe)",
                    ReplaceInputMode::Recycle => "Move to recycle bin",
                    ReplaceInputMode::Delete => "Delete permanently (DANGER!)",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.replace_input, ReplaceInputMode::Off, "Keep original files (safe)");
                    ui.selectable_value(&mut self.replace_input, ReplaceInputMode::Recycle, "Move to recycle bin");
                    ui.selectable_value(&mut self.replace_input, ReplaceInputMode::Delete, "Delete permanently (DANGER!)");
                });
                
            if self.replace_input != ReplaceInputMode::Off {
                ui.colored_label(egui::Color32::ORANGE, "⚠️ Warning: This will modify/remove your original files!");
            }
        });

        ui.add_space(15.0);

        // Testing & Validation
        ui.group(|ui| {
            ui.label("🧪 Testing & Validation");
            ui.add_space(5.0);
            
            ui.checkbox(&mut self.dry_run, "Dry run mode (preview only, no actual conversion)");
            
            if self.dry_run {
                ui.colored_label(egui::Color32::BLUE, "ℹ️ Dry run mode: No files will be modified");
            }
        });

        ui.add_space(15.0);

        // Logging & Output
        ui.group(|ui| {
            ui.label("📝 Logging & Output");
            ui.add_space(5.0);
            
            ui.checkbox(&mut self.verbose, "Verbose logging (detailed output)");
            ui.checkbox(&mut self.quiet, "Quiet mode (minimal output)");
            
            if self.verbose && self.quiet {
                ui.colored_label(egui::Color32::ORANGE, "⚠️ Verbose and Quiet modes conflict - Verbose will take priority");
                self.quiet = false;
            }
        });

        ui.add_space(15.0);

        // Report Generation
        ui.group(|ui| {
            ui.label("📊 Report Generation");
            ui.add_space(5.0);
            
            ui.checkbox(&mut self.generate_report, "Generate conversion report");
            
            if self.generate_report {
                ui.horizontal(|ui| {
                    ui.label("Report format:");
                    egui::ComboBox::from_id_salt("report_format")
                        .selected_text(match self.report_format {
                            ReportFormat::Json => "JSON",
                            ReportFormat::Csv => "CSV",
                            ReportFormat::Html => "HTML",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.report_format, ReportFormat::Json, "JSON");
                            ui.selectable_value(&mut self.report_format, ReportFormat::Csv, "CSV");
                            ui.selectable_value(&mut self.report_format, ReportFormat::Html, "HTML");
                        });
                });
            }
        });

        ui.add_space(15.0);

        // Configuration Management
        ui.group(|ui| {
            ui.label("⚙️ Configuration Management");
            ui.add_space(5.0);
            
            ui.horizontal(|ui| {
                ui.label("Config file:");
                ui.add(egui::TextEdit::singleline(&mut self.config_file)
                    .hint_text("Path to configuration file"));
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Select Configuration File")
                        .add_filter("TOML", &["toml"])
                        .pick_file() {
                        self.config_file = path.display().to_string();
                    }
                }
            });
            
            ui.horizontal(|ui| {
                ui.label("Profile:");
                ui.add(egui::TextEdit::singleline(&mut self.profile)
                    .hint_text("Configuration profile name"));
            });
        });
    }

    fn show_progress_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(&Icons::with_text(Icons::PROGRESS, "Conversion Progress"));
        ui.add_space(10.0);

        if self.is_converting || self.total_files > 0 {
            // Progress overview
            ui.group(|ui| {
                ui.label("📈 Progress Overview");
                ui.add_space(5.0);

                if self.is_converting {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Conversion in progress...");
                    });
                }

                ui.add(egui::ProgressBar::new(self.progress)
                    .text(format!("{}/{} files processed", self.processed_files, self.total_files)));

                ui.horizontal(|ui| {
                    ui.label(format!("✅ Processed: {}", self.processed_files));
                    if self.failed_files > 0 {
                        ui.colored_label(egui::Color32::RED, format!("❌ Failed: {}", self.failed_files));
                    }
                    let remaining = self.total_files.saturating_sub(self.processed_files + self.failed_files);
                    if remaining > 0 {
                        ui.label(format!("⏳ Remaining: {}", remaining));
                    }
                });
            });

            ui.add_space(15.0);

            // Conversion Log
            if !self.conversion_log.is_empty() {
                ui.group(|ui| {
                    ui.label("📝 Conversion Log");
                    ui.add_space(5.0);
                    
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for log_entry in &self.conversion_log {
                                ui.label(log_entry);
                            }
                        });
                });
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No conversion in progress. Go to Input & Output tab to get started.");
            });
        }
    }

    fn show_results_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(&Icons::with_text(Icons::RESULTS, "Conversion Results"));
        ui.add_space(10.0);

        // Error Display
        if let Some(error) = &self.error_message {
            ui.group(|ui| {
                ui.colored_label(egui::Color32::RED, "❌ Error");
                ui.add_space(5.0);
                ui.label(error);
            });
            ui.add_space(15.0);
        }

        // Results Summary
        if let Some(report) = &self.last_report {
            // File Statistics
            ui.group(|ui| {
                ui.label("📊 File Statistics");
                ui.add_space(5.0);

                egui::Grid::new("file_stats")
                    .num_columns(2)
                    .spacing([20.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("✅ Processed:");
                        ui.label(format!("{} files", report.processed_files));
                        ui.end_row();

                        if report.failed_files > 0 {
                            ui.colored_label(egui::Color32::RED, "❌ Failed:");
                            ui.colored_label(egui::Color32::RED, format!("{} files", report.failed_files));
                            ui.end_row();
                        }

                        if report.skipped_files > 0 {
                            ui.label("⏭️ Skipped:");
                            ui.label(format!("{} files", report.skipped_files));
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(15.0);

            // Space Analysis
            if report.original_size > 0 {
                ui.group(|ui| {
                    ui.label("💾 Space Analysis");
                    ui.add_space(5.0);

                    egui::Grid::new("space_stats")
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            ui.label("📦 Original size:");
                            ui.label(humansize::format_size(report.original_size, humansize::DECIMAL));
                            ui.end_row();

                            ui.label("🗜️ Compressed size:");
                            ui.label(humansize::format_size(report.compressed_size, humansize::DECIMAL));
                            ui.end_row();

                            ui.label("💾 Space saved:");
                            let savings = report.original_size.saturating_sub(report.compressed_size);
                            ui.label(format!("{} ({:.1}%)", 
                                humansize::format_size(savings, humansize::DECIMAL),
                                report.compression_ratio * 100.0));
                            ui.end_row();
                        });
                });

                ui.add_space(15.0);
            }

            // Performance Metrics
            ui.group(|ui| {
                ui.label("⏱️ Performance Metrics");
                ui.add_space(5.0);

                egui::Grid::new("perf_stats")
                    .num_columns(2)
                    .spacing([20.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("🕐 Duration:");
                        ui.label(format!("{:.1} seconds", report.duration.as_secs_f64()));
                        ui.end_row();

                        ui.label("🚀 Processing speed:");
                        ui.label(format!("{:.1} files/second", report.files_per_second));
                        ui.end_row();

                        ui.label("🧵 Threads used:");
                        ui.label(format!("{}", report.thread_count));
                        ui.end_row();
                    });
            });

            ui.add_space(15.0);

            // Export Results
            ui.group(|ui| {
                ui.label("📤 Export Results");
                ui.add_space(5.0);
                
                ui.horizontal(|ui| {
                    if ui.button("📄 Generate JSON Report").clicked() {
                        if let Err(e) = webpify::generate_report(report, &ReportFormat::Json) {
                            self.error_message = Some(format!("Failed to generate report: {}", e));
                        }
                    }
                    
                    if ui.button("📊 Generate CSV Report").clicked() {
                        if let Err(e) = webpify::generate_report(report, &ReportFormat::Csv) {
                            self.error_message = Some(format!("Failed to generate report: {}", e));
                        }
                    }
                    
                    if ui.button("🌐 Generate HTML Report").clicked() {
                        if let Err(e) = webpify::generate_report(report, &ReportFormat::Html) {
                            self.error_message = Some(format!("Failed to generate report: {}", e));
                        }
                    }
                });
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No results yet. Run a conversion to see detailed results here.");
            });
        }
    }
    
    fn generate_preview(&mut self) {
        // Clear any previous error messages first
        self.error_message = None;
        
        if self.input_dir.is_empty() {
            self.error_message = Some("Input directory is empty".to_string());
            return;
        }
        
        self.preview_files.clear();
        let input_path = PathBuf::from(&self.input_dir);
        
        // Validate input path exists
        if !input_path.exists() {
            self.error_message = Some("Input directory does not exist".to_string());
            return;
        }
        
        if !input_path.is_dir() {
            self.error_message = Some("Input path is not a directory".to_string());
            return;
        }
        
        // Parse supported formats
        let formats: Vec<String> = self.formats
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        
        if formats.is_empty() {
            self.error_message = Some("No file formats specified".to_string());
            return;
        }
        
        // Recursively scan directory for supported files with error handling
        match self.scan_directory_safe(&input_path, &formats) {
            Ok(_) => {
                // Sort by file size (largest first) for better overview
                self.preview_files.sort_by(|a, b| b.size.cmp(&a.size));
                
                // Limit to first 100 files for performance
                self.preview_files.truncate(100);
                
                // If no files found, show helpful message
                if self.preview_files.is_empty() {
                    self.error_message = Some(format!("No files with supported formats ({}) found in directory", self.formats));
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Error scanning directory: {}", e));
            }
        }
    }
    
    fn scan_directory_safe(&mut self, dir: &PathBuf, formats: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        
        // Safety check to prevent infinite recursion or too deep scanning
        if self.preview_files.len() >= 100 {
            return Ok(()); // Stop scanning if we already have enough files
        }
        
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                // Instead of printing to stderr, return a proper error for GUI handling
                return Err(format!("Cannot read directory {}: {}", dir.display(), e).into());
            }
        };
        
        for entry in entries {
            if self.preview_files.len() >= 100 {
                break; // Stop if we have enough files
            }
            
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("Warning: Error reading directory entry: {}", e);
                    continue;
                }
            };
            
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(e) => {
                    eprintln!("Warning: Cannot determine file type for {}: {}", entry.path().display(), e);
                    continue;
                }
            };
            
            if file_type.is_file() {
                let path = entry.path();
                if let Some(extension) = path.extension() {
                    let ext = extension.to_string_lossy().to_lowercase();
                    if formats.contains(&ext) {
                        match fs::metadata(&path) {
                            Ok(metadata) => {
                                // Estimate output size based on compression mode and quality
                                let estimated_size = self.estimate_webp_size(metadata.len());
                                
                                self.preview_files.push(PreviewFileInfo {
                                    path: path.clone(),
                                    size: metadata.len(),
                                    format: ext,
                                    estimated_output_size: Some(estimated_size),
                                });
                            }
                            Err(e) => {
                                // Log the error but continue processing other files
                                eprintln!("Warning: Could not read metadata for {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            } else if file_type.is_dir() && self.preserve_structure {
                // Recursively scan subdirectories if preserve_structure is enabled
                // Use a depth limit to prevent infinite recursion
                if let Some(depth) = self.get_directory_depth(&entry.path(), dir) {
                    if depth < 5 { // Reduced depth limit for safety
                        if let Err(e) = self.scan_directory_safe(&entry.path(), formats) {
                            eprintln!("Warning: Error scanning subdirectory {}: {}", entry.path().display(), e);
                            // Continue with other directories instead of failing completely
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn get_directory_depth(&self, path: &PathBuf, base: &PathBuf) -> Option<usize> {
        path.strip_prefix(base).ok().map(|p| p.components().count())
    }
    
    fn estimate_webp_size(&self, original_size: u64) -> u64 {
        // Rough estimation based on compression mode and quality
        let compression_factor = match self.mode {
            CompressionMode::Lossless => 0.7, // Lossless typically saves 20-30%
            CompressionMode::Lossy => {
                // Lossy compression factor based on quality
                match self.quality {
                    90..=100 => 0.6,
                    70..=89 => 0.4,
                    50..=69 => 0.3,
                    _ => 0.2,
                }
            },
            CompressionMode::Auto => 0.5, // Conservative estimate for auto mode
        };
        
        (original_size as f64 * compression_factor) as u64
    }
    
    fn show_preview_modal(&mut self, ctx: &egui::Context) {
        if !self.show_preview_window {
            return;
        }
        
        egui::Window::new("🔍 Conversion Preview")
            .default_width(700.0)
            .default_height(500.0)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("📊 Found {} files to convert", self.preview_files.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("❌ Close").clicked() {
                            self.show_preview_window = false;
                        }
                        if !self.preview_files.is_empty() && ui.button("🚀 Start Conversion").clicked() {
                            self.show_preview_window = false;
                            self.start_conversion();
                        }
                    });
                });
                
                ui.separator();
                
                // Show error if any occurred during scanning
                if let Some(error) = &self.error_message {
                    ui.group(|ui| {
                        ui.colored_label(egui::Color32::RED, "❌ Error");
                        ui.add_space(5.0);
                        ui.label(error);
                    });
                    ui.add_space(15.0);
                }
                
                if self.preview_files.is_empty() {
                    ui.centered_and_justified(|ui| {
                        if self.error_message.is_some() {
                            ui.label("Preview generation failed. Please check the error above and try again.");
                        } else {
                            ui.label("No supported files found in the selected directory.");
                        }
                    });
                    return;
                }
                
                // Summary statistics
                ui.group(|ui| {
                    ui.label("📈 Summary");
                    ui.add_space(5.0);
                    
                    let total_size: u64 = self.preview_files.iter().map(|f| f.size).sum();
                    let estimated_output: u64 = self.preview_files.iter()
                        .filter_map(|f| f.estimated_output_size)
                        .sum();
                    let estimated_savings = total_size.saturating_sub(estimated_output);
                    let savings_percent = if total_size > 0 {
                        (estimated_savings as f64 / total_size as f64) * 100.0
                    } else {
                        0.0
                    };
                    
                    egui::Grid::new("preview_summary")
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            ui.label("🗂️ Total files:");
                            ui.label(format!("{}", self.preview_files.len()));
                            ui.end_row();
                            
                            ui.label("📦 Total size:");
                            ui.label(humansize::format_size(total_size, humansize::DECIMAL));
                            ui.end_row();
                            
                            ui.label("🗜️ Estimated output:");
                            ui.label(humansize::format_size(estimated_output, humansize::DECIMAL));
                            ui.end_row();
                            
                            ui.label("💾 Estimated savings:");
                            ui.colored_label(
                                egui::Color32::GREEN,
                                format!("{} ({:.1}%)", 
                                    humansize::format_size(estimated_savings, humansize::DECIMAL),
                                    savings_percent
                                )
                            );
                            ui.end_row();
                        });
                });
                
                ui.add_space(10.0);
                
                // File list with details
                ui.group(|ui| {
                    ui.label("📋 File Details");
                    ui.add_space(5.0);
                    
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            egui::Grid::new("preview_files")
                                .num_columns(4)
                                .spacing([10.0, 2.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    // Header
                                    ui.strong("File");
                                    ui.strong("Format");
                                    ui.strong("Size");
                                    ui.strong("Est. Output");
                                    ui.end_row();
                                    
                                    for file in &self.preview_files {
                                        // File name (truncated if too long)
                                        let file_name = file.path.file_name()
                                            .map(|n| n.to_string_lossy())
                                            .unwrap_or_else(|| "Unknown".into());
                                        
                                        let display_name = if file_name.chars().count() > 30 {
                                            let truncated: String = file_name.chars().take(27).collect();
                                            format!("{}...", truncated)
                                        } else {
                                            file_name.to_string()
                                        };
                                        
                                        ui.label(display_name);
                                        ui.label(file.format.to_uppercase());
                                        ui.label(humansize::format_size(file.size, humansize::DECIMAL));
                                        
                                        if let Some(output_size) = file.estimated_output_size {
                                            let savings = file.size.saturating_sub(output_size);
                                            let savings_percent = if file.size > 0 {
                                                (savings as f64 / file.size as f64) * 100.0
                                            } else {
                                                0.0
                                            };
                                            ui.colored_label(
                                                egui::Color32::DARK_GREEN,
                                                format!("{} (-{:.0}%)", 
                                                    humansize::format_size(output_size, humansize::DECIMAL),
                                                    savings_percent
                                                )
                                            );
                                        } else {
                                            ui.label("Unknown");
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                });
                
                if self.preview_files.len() >= 100 {
                    ui.add_space(5.0);
                    ui.colored_label(egui::Color32::ORANGE, 
                        "⚠️ Showing first 100 files only. Use filters to refine your selection.");
                }
            });
    }
    
    fn show_help_modal(&mut self, ctx: &egui::Context) {
        if !self.show_help_window {
            return;
        }
        
        egui::Window::new("💡 Help & Guide")
            .default_width(600.0)
            .default_height(400.0)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Webpify Help").size(18.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("❌ Close").clicked() {
                            self.show_help_window = false;
                        }
                    });
                });
                
                ui.separator();
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Context-sensitive help based on current tab
                    match self.current_tab {
                        Tab::Input => self.show_input_help(ui),
                        Tab::Settings => self.show_settings_help(ui),
                        Tab::Advanced => self.show_advanced_help(ui),
                        Tab::Progress => self.show_progress_help(ui),
                        Tab::Results => self.show_results_help(ui),
                    }
                    
                    ui.add_space(20.0);
                    
                    // General help sections
                    ui.collapsing("🚀 Quick Start Guide", |ui| {
                        ui.label("1. 📁 Select an input directory containing your images");
                        ui.label("2. ⚙️ Configure quality and compression settings");
                        ui.label("3. 🔧 (Optional) Adjust advanced options");
                        ui.label("4. 🔍 Preview your conversion to verify settings");
                        ui.label("5. ▶️ Start the conversion process");
                        ui.label("6. 📊 Review results and generate reports");
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.collapsing("🎯 Quality Settings Guide", |ui| {
                        ui.label("🏆 Best (95%): Perfect for archival, professional work");
                        ui.label("⚖️ Balanced (85%): Good compromise between quality and size");
                        ui.label("📦 Small (70%): Suitable for web usage, social media");
                        ui.label("🗜️ Tiny (50%): Maximum compression for bandwidth-limited scenarios");
                        ui.add_space(5.0);
                        ui.label(egui::RichText::new("💡 Tip: Use Preview to test different settings!").italics());
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.collapsing("📁 Supported Formats", |ui| {
                        ui.label("✅ Input: JPEG, PNG, GIF, BMP, TIFF, WebP");
                        ui.label("✅ Output: WebP (modern, efficient format)");
                        ui.add_space(5.0);
                        ui.label("🌟 WebP benefits:");
                        ui.label("  • 25-35% smaller than JPEG");
                        ui.label("  • Supports transparency like PNG");
                        ui.label("  • Excellent browser support");
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.collapsing("⚡ Performance Tips", |ui| {
                        ui.label("🧵 Use auto-thread detection for optimal performance");
                        ui.label("💾 SSD storage significantly improves conversion speed");
                        ui.label("🔍 Enable pre-scan for accurate progress estimates");
                        ui.label("📊 Use batch processing for large image collections");
                        ui.label("🗂️ Preserve directory structure for organized output");
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.collapsing("🛡️ Safety Features", |ui| {
                        ui.label("🔍 Dry run mode: Preview changes without modifying files");
                        ui.label("🗑️ Recycle bin: Safely remove originals (can be restored)");
                        ui.label("📊 Detailed reports: Track all conversions and errors");
                        ui.label("✅ Validation: Automatic checks before conversion starts");
                        ui.label("💾 Backup recommendation: Always backup important files first");
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.collapsing("❓ Troubleshooting", |ui| {
                        ui.label("🚫 \"No files found\": Check file formats and directory permissions");
                        ui.label("💥 \"Conversion failed\": Verify sufficient disk space and file access");
                        ui.label("🐌 \"Slow performance\": Reduce thread count or check disk speed");
                        ui.label("📁 \"Directory errors\": Ensure input/output paths are valid");
                        ui.label("🔧 \"Settings issues\": Use presets to reset to known good values");
                    });
                });
            });
    }
    
    fn show_input_help(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("📁 Input & Output Help").size(16.0).strong());
            ui.add_space(5.0);
            
            ui.label("🎯 Current Step: Configure input and output directories");
            ui.add_space(8.0);
            
            ui.label("📂 Input Directory:");
            ui.label("  • Select folder containing images to convert");
            ui.label("  • Supports nested subdirectories");
            ui.label("  • Only supported formats will be processed");
            
            ui.add_space(5.0);
            ui.label("📁 Output Directory:");
            ui.label("  • Auto mode creates 'webp_output' subfolder");
            ui.label("  • Custom mode allows any destination");
            ui.label("  • Directory structure can be preserved");
            
            ui.add_space(5.0);
            ui.label("🎯 File Formats:");
            ui.label("  • Use preset buttons for common scenarios");
            ui.label("  • Comma-separated list (jpg,png,gif...)");
            ui.label("  • Case insensitive matching");
        });
    }
    
    fn show_settings_help(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("⚙️ Settings Help").size(16.0).strong());
            ui.add_space(5.0);
            
            ui.label("🎯 Current Step: Configure conversion quality and performance");
            ui.add_space(8.0);
            
            ui.label("🏆 Quality Presets:");
            ui.label("  • Click presets for instant configuration");
            ui.label("  • Slider allows fine-tuning (1-100%)");
            ui.label("  • Higher quality = larger files but better image");
            
            ui.add_space(5.0);
            ui.label("🤖 Compression Modes:");
            ui.label("  • Lossless: Perfect quality, moderate compression");
            ui.label("  • Lossy: Good quality, maximum compression");
            ui.label("  • Auto: Automatically chooses best mode per image");
            
            ui.add_space(5.0);
            ui.label("⚡ Performance:");
            ui.label("  • More threads = faster conversion");
            ui.label("  • Auto-detection recommended for most users");
            ui.label("  • Pre-scan provides accurate progress estimates");
        });
    }
    
    fn show_advanced_help(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("🔧 Advanced Help").size(16.0).strong());
            ui.add_space(5.0);
            
            ui.label("🎯 Current Step: Fine-tune advanced conversion options");
            ui.add_space(8.0);
            
            ui.label("🗂️ File Handling:");
            ui.label("  • Keep originals (safest option)");
            ui.label("  • Recycle bin (can be restored)");
            ui.label("  • Permanent delete (cannot be undone!)");
            
            ui.add_space(5.0);
            ui.label("🧪 Testing:");
            ui.label("  • Dry run shows what will happen without changes");
            ui.label("  • Perfect for testing settings on large collections");
            
            ui.add_space(5.0);
            ui.label("📊 Reports:");
            ui.label("  • JSON: Machine-readable format");
            ui.label("  • CSV: Spreadsheet-compatible");
            ui.label("  • HTML: Human-readable with charts");
        });
    }
    
    fn show_progress_help(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("📊 Progress Help").size(16.0).strong());
            ui.add_space(5.0);
            
            ui.label("🎯 Current Step: Monitor conversion progress");
            ui.add_space(8.0);
            
            ui.label("📈 Progress Indicators:");
            ui.label("  • Progress bar shows overall completion");
            ui.label("  • File counters track processed/failed/remaining");
            ui.label("  • Real-time log shows current activity");
            
            ui.add_space(5.0);
            ui.label("⏹️ Controls:");
            ui.label("  • Stop button halts conversion safely");
            ui.label("  • Conversion can be resumed by starting again");
            ui.label("  • Existing WebP files are skipped unless overwrite is enabled");
        });
    }
    
    fn show_results_help(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("📈 Results Help").size(16.0).strong());
            ui.add_space(5.0);
            
            ui.label("🎯 Current Step: Review conversion results and export reports");
            ui.add_space(8.0);
            
            ui.label("📊 Statistics:");
            ui.label("  • File counts show success/failure rates");
            ui.label("  • Size analysis shows space savings");
            ui.label("  • Performance metrics help optimize future runs");
            
            ui.add_space(5.0);
            ui.label("📤 Export Options:");
            ui.label("  • Reports contain detailed conversion information");
            ui.label("  • Use for documentation or analysis");
            ui.label("  • Choose format based on intended use");
        });
    }

    fn start_conversion(&mut self) {
        // Validate input
        if self.input_dir.is_empty() {
            self.error_message = Some("Please select an input directory".to_string());
            return;
        }

        let input_path = PathBuf::from(&self.input_dir);
        if !input_path.exists() {
            self.error_message = Some("Input directory does not exist".to_string());
            return;
        }

        // Parse threads
        let threads = if self.threads_auto {
            None
        } else {
            match self.threads.parse::<usize>() {
                Ok(t) if t > 0 => Some(t),
                _ => {
                    self.error_message = Some("Invalid thread count".to_string());
                    return;
                }
            }
        };

        // Parse max size
        let max_size_mb = if self.max_size.is_empty() {
            None
        } else {
            match self.max_size.parse::<u64>() {
                Ok(size) => Some(size),
                _ => {
                    self.error_message = Some("Invalid maximum file size".to_string());
                    return;
                }
            }
        };

        // Clear previous results
        self.clear_results();
        self.is_converting = true;
        self.current_tab = Tab::Progress; // Auto-switch to progress tab

        // Create conversion options with full configuration
        let mut options = ConversionOptions::new(input_path)
            .with_quality(self.quality)
            .with_mode(self.mode.clone())
            .with_dry_run(self.dry_run)
            .with_overwrite(self.overwrite)
            .with_preserve_structure(self.preserve_structure)
            .with_min_size_kb(self.min_size)
            .with_prescan(self.prescan)
            .with_reencode_webp(self.reencode_webp)
            .with_replace_input_mode(self.replace_input.clone());

        // Set output directory
        if self.output_dir_auto || self.output_dir.is_empty() {
            // Use default output directory (input_dir/webp_output)
            let mut default_output = PathBuf::from(&self.input_dir);
            default_output.push("webp_output");
            options = options.with_output_dir(default_output);
        } else {
            options = options.with_output_dir(PathBuf::from(&self.output_dir));
        }

        // Set thread count
        if let Some(threads) = threads {
            options = options.with_threads(threads);
        }

        // Set max file size
        if let Some(max_size) = max_size_mb {
            options = options.with_max_size_mb(max_size);
        }

        // Parse and set supported formats
        let formats: Vec<String> = self.formats
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        
        if !formats.is_empty() {
            options = options.with_supported_formats(formats);
        }

        // Start conversion in background thread
        let progress_reporter = Arc::clone(&self.progress_reporter);
        let generate_report = self.generate_report;
        let report_format = self.report_format.clone();
        
        thread::spawn(move || {
            let mut core = WebpifyCore::new(options);
            
            // Create progress reporter
            let reporter: Box<dyn ProgressReporter> = Box::new(ThreadSafeGuiProgressReporter {
                inner: Arc::clone(&progress_reporter),
            });

            match core.run_with_progress(Some(reporter)) {
                Ok(report) => {
                    // Generate report if requested
                    if generate_report {
                        if let Err(e) = webpify::generate_report(&report, &report_format) {
                            if let Ok(mut progress) = progress_reporter.lock() {
                                progress.error = Some(format!("Conversion succeeded but failed to generate report: {}", e));
                                progress.report = Some(report);
                                progress.finished = true;
                            }
                            return;
                        }
                    }
                    
                    if let Ok(mut progress) = progress_reporter.lock() {
                        progress.report = Some(report);
                        progress.finished = true;
                    }
                }
                Err(e) => {
                    if let Ok(mut progress) = progress_reporter.lock() {
                        progress.error = Some(format!("{:#}", e));
                        progress.finished = true;
                    }
                }
            }
        });
    }

    fn clear_results(&mut self) {
        self.last_report = None;
        self.error_message = None;
        self.progress = 0.0;
        self.total_files = 0;
        self.processed_files = 0;
        self.failed_files = 0;
        self.conversion_log.clear();
        
        if let Ok(mut reporter) = self.progress_reporter.lock() {
            *reporter = GuiProgressReporter::new();
        }
    }
}

/// Progress reporter that can be safely shared between threads
struct GuiProgressReporter {
    total_files: usize,
    processed_files: usize,
    failed_files: usize,
    finished: bool,
    report: Option<ConversionReport>,
    error: Option<String>,
    logs: Vec<String>,
}

impl GuiProgressReporter {
    fn new() -> Self {
        Self {
            total_files: 0,
            processed_files: 0,
            failed_files: 0,
            finished: false,
            report: None,
            error: None,
            logs: Vec::new(),
        }
    }
}

/// Thread-safe wrapper for GUI progress reporter
struct ThreadSafeGuiProgressReporter {
    inner: Arc<Mutex<GuiProgressReporter>>,
}

impl ProgressReporter for ThreadSafeGuiProgressReporter {
    fn set_total_files(&self, total: usize) {
        if let Ok(mut reporter) = self.inner.lock() {
            reporter.total_files = total;
        }
    }

    fn update_progress(&self, processed: usize, failed: usize) {
        if let Ok(mut reporter) = self.inner.lock() {
            reporter.processed_files = processed;
            reporter.failed_files = failed;
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you want to see logs)

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Webpify - Batch WebP Converter",
        options,
        Box::new(|cc| {
            // Setup fonts for better international and emoji support
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(WebpifyGuiApp::default()))
        }),
    )
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    
    // Install system fonts for better international character support
    // This allows proper display of CJK characters in file paths and names,
    // but the UI itself remains in English
    #[cfg(target_os = "windows")]
    {
        // Try to load Windows system fonts for better international support
        if let Ok(font_data) = std::fs::read("C:/Windows/Fonts/msyh.ttc") {
            fonts.font_data.insert(
                "microsoft_yahei".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
                .insert(1, "microsoft_yahei".to_owned()); // Insert after default font
        } else if let Ok(font_data) = std::fs::read("C:/Windows/Fonts/simsun.ttc") {
            fonts.font_data.insert(
                "simsun".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
                .insert(1, "simsun".to_owned());
        }
        
        // Try to load Segoe UI Emoji for better emoji support
        if let Ok(font_data) = std::fs::read("C:/Windows/Fonts/seguiemj.ttf") {
            fonts.font_data.insert(
                "segoe_ui_emoji".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
                .push("segoe_ui_emoji".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}
