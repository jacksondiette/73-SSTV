use eframe::egui;
use egui::{ColorImage, RichText, TextureHandle, Visuals};
use rfd::{self, FileDialog};
use num_complex::{Complex, c32};
use std::thread;
use std::sync::{Arc, Mutex};
use image::{ImageBuffer, Rgba};

pub mod fft;
pub mod freq_to_img;
pub mod img_to_freq;

struct Globals {
    show_decode_panel: bool,
    decode_load_path: Option<String>,
    encode_load_path: Option<String>,
    frequency_mutex: Arc<Mutex<Vec<f32>>>,
    main_image: Option<ColorImage>,
    encode_image: Option<ColorImage>,
    sound_buffer: Option<Vec<i16>>,
    main_texture_handle: Option<TextureHandle>,
    pending_image_decode: Arc<Mutex<bool>>,
    decode_mode: Mode,
    encode_mode: Mode,
    is_decoding: bool,
    program_status: Arc<Mutex<String>>
}

#[derive(Clone)]
#[derive(PartialEq)]
pub enum Mode {
    RAW,
    MartinM1,
    MartinM2,
    //Scottie1,
    PD120,
    PD180,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Mode::RAW => "Raw / BW",
            Mode::MartinM1 => "Martin M1",
            Mode::MartinM2 => "Martin M2",
            //Mode::Scottie1 => "Scottie 1",
            Mode::PD120 => "PD 120",
            Mode::PD180 => "PD 180",
        };
        write!(f, "{label}")
    }
}

impl Default for Globals {
    fn default() -> Self {

        Self { 
            show_decode_panel: true,
            decode_load_path: None,
            encode_load_path: None,
            frequency_mutex: Arc::new(Mutex::new(vec![0.0])),
            main_image: None,
            encode_image: None,
            sound_buffer: None,
            main_texture_handle: None,
            pending_image_decode: Arc::new(Mutex::new(false)),
            decode_mode: Mode::RAW,
            encode_mode: Mode::MartinM1,
            is_decoding: false,
            program_status: Arc::new(Mutex::new(String::from("Waiting...")))
        }
    }
}

impl eframe::App for Globals {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(Visuals::dark());

        if *self.pending_image_decode.lock().unwrap() {
            *self.pending_image_decode.lock().unwrap() = false;
            self.main_image = Some(freq_to_img::decode_image(self.frequency_mutex.lock().unwrap().clone(), self.decode_mode.clone()));
            self.main_texture_handle = None;
            *self.program_status.lock().unwrap() = String::from("Done!");
        }

        if let Some(image_data) = &self.main_image {
            if self.main_texture_handle.is_none() {
                self.main_texture_handle = Some(ctx.load_texture("main_image", image_data.clone(), egui::TextureOptions::NEAREST))
            }
        }

        egui::TopBottomPanel::top("top_menu").frame(egui::Frame::new().fill(egui::Color32::from_rgb(20, 20, 21)).inner_margin(10.0)).show(ctx, |ui|{
            ui.horizontal(|ui| {
                ui.heading(RichText::new("73-SSTV").size(24.0));
                if ui.selectable_label(false, "Decoding").clicked() {
                    self.show_decode_panel = true;
                }
                if ui.selectable_label(false, "Encoding").clicked() {
                    self.show_decode_panel = false;
                }
            });

            ui.separator()
        });

        egui::SidePanel::left("side bar").min_width(150.0).frame(egui::Frame::new().fill(egui::Color32::from_rgb(20, 20, 21)).inner_margin(15.0)).show(ctx, |ui|{ 
            if self.show_decode_panel {
                ui.heading(RichText::new("Decoding").size(32.0));
                ui.separator();
                ui.heading(RichText::new("Input").size(32.0));

                if ui.add_sized(egui::vec2(120.0, 30.0), egui::Button::new(RichText::new("Select File 📁"))).clicked() {
                    if let Some(file_path) = FileDialog::new().add_filter(".wav File", &["wav"]).pick_file() {
                        self.decode_load_path = Some(file_path.display().to_string());
                    }
                }

                if let Some(file_path) = &self.decode_load_path {
                    ui.label(format!("File: {}", file_path));

                    if ui.add_sized(egui::vec2(120.0, 30.0), egui::Button::new(RichText::new("Decode ⚡").strong())).clicked() && !self.is_decoding {
                        
                        let file_path_clone = file_path.clone();
                        let freq_buffer = self.frequency_mutex.clone();
                        let pending_decode = self.pending_image_decode.clone();
                        let status = self.program_status.clone();
                        thread::spawn(move || {
                            let set_status = |new_text: &str| {
                                *status.lock().unwrap() = new_text.to_string();
                            };

                            set_status("Reading File...");

                            let mut file_reader = hound::WavReader::open(file_path_clone).expect("Error Opening File");
                            let file_specs = file_reader.spec();
                            println!("{:?}", file_specs);

                            let mut samples: Vec<Complex<f32>>;

                            if file_specs.sample_format == hound::SampleFormat::Float {
                                samples = file_reader.samples::<f32>()
                                .map(|z| c32(z.unwrap() / (i32::MAX as f32),0.0))
                                .collect();
                            } else {
                                samples = file_reader.samples::<i32>()
                                .map(|z| c32(z.unwrap() as f32 / (i32::MAX as f32),0.0))
                                .collect();
                            }
                            if file_specs.channels == 2 {
                                samples = samples.chunks(2)
                                .map(|pair| 0.5 * (pair[0] + pair[1]))
                                .collect()
                            }

                            set_status("Performing Hilbert Transform...");
                                            
                            let iq_samples = fft::hilbert(samples, file_specs.sample_rate as f32, 900.0, 2500.0);

                            set_status("Calculating Phase...");

                            let mut freqs: Vec<f32> = Vec::with_capacity(iq_samples.len()-1);

                            for i in 1..iq_samples.len() {
                                let diff = Complex::arg(iq_samples[i] * Complex::conj(&iq_samples[i-1]));
                                freqs.push(f32::abs(diff*(file_specs.sample_rate as f32))/(2.0*std::f32::consts::PI));
                            }

                            *freq_buffer.lock().unwrap() = freqs;
                            *pending_decode.lock().unwrap() = true;

                            set_status("Building Image...");
                        });

                    }
                }

                ui.separator();
                ui.heading(RichText::new("Processing").size(32.0));

                egui::ComboBox::from_label("Selected Mode").selected_text(self.decode_mode.to_string()).show_ui(ui, |ui| {
                    for option in [
                        Mode::RAW, 
                        Mode::MartinM1, 
                        Mode::MartinM2,
                        Mode::PD120, 
                        Mode::PD180,
                        ] {
                        if ui.selectable_value(&mut self.decode_mode, option.clone(), option.to_string()).clicked() {
                            self.decode_mode = option;
                        }
                    }
                });

                if self.frequency_mutex.lock().unwrap().len() > 1 {
                    if ui.add_sized(egui::vec2(120.0, 30.0), egui::Button::new(RichText::new("Recalculate Image ↻"))).clicked() {
                        self.main_image = Some(freq_to_img::decode_image(self.frequency_mutex.lock().unwrap().clone(), self.decode_mode.clone()));
                        self.main_texture_handle = None;
                    };
                }

                ui.separator();
                ui.heading(RichText::new("Output").size(32.0));

                if let Some(image_data) = &self.main_image {
                    if ui.add_sized(egui::vec2(120.0, 30.0), egui::Button::new(RichText::new("Save Image 💾"))).clicked() {
                        let w = image_data.width();
                        let h = image_data.height();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Image File", &["png"])
                            .save_file()
                        {
                            let buffer: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(w as u32, h as u32, image_data.as_raw()).expect("Invalid Image Data");
                            let _ = buffer.save(path);
                        }
                    }
                }

            } else {
                ui.heading(RichText::new("Encoding").size(32.0));
                ui.separator();
                ui.heading(RichText::new("Input").size(32.0));

                if ui.add_sized(egui::vec2(120.0, 30.0), egui::Button::new(RichText::new("Select File 📁"))).clicked() {
                    if let Some(path) = FileDialog::new().add_filter("Image File", &["png"]).pick_file() {
                        self.encode_load_path = Some(path.display().to_string());
                        let rgba = image::ImageReader::open(path).expect("Image Reader Failed").decode().expect("Image Decoding Failed").to_rgba8();
                        let (width, height) = rgba.dimensions();
                        let pixels: Vec<egui::Color32> = rgba.pixels()
                            .map(|p| egui::Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3]))
                            .collect();
                        let new_image = egui::ColorImage {
                            size: [width as usize, height as usize],
                            source_size: egui::Vec2 { x: (width as f32), y: (height as f32) },
                            pixels,
                        };
                        self.encode_image = Some(new_image);
                        self.main_image = self.encode_image.clone();
                        self.main_texture_handle = None;
                    }
                }

                if let Some(file_path) = &self.encode_load_path {
                    ui.label(format!("File: {}", file_path)); 

                    if self.encode_image.is_some() {
                        if ui.add_sized(egui::vec2(120.0, 30.0), egui::Button::new(RichText::new("Encode ⚡").strong())).clicked() {
                            self.sound_buffer = Some(img_to_freq::encode(self.encode_image.clone().expect("Image Copy Fail"), self.encode_mode.clone()));
                        }
                    }
                }

                ui.separator();
                ui.heading(RichText::new("Processing").size(32.0));

                egui::ComboBox::from_label("Selected Mode").selected_text(self.encode_mode.to_string()).show_ui(ui, |ui| {
                    for option in [
                        Mode::MartinM1, 
                        Mode::MartinM2,
                        ] {
                        if ui.selectable_value(&mut self.encode_mode, option.clone(), option.to_string()).clicked() {
                            self.encode_mode = option;
                        }
                    }
                });

                ui.separator();
                ui.heading(RichText::new("Output").size(32.0));

                if let Some(sound_samples) = &self.sound_buffer {
                    if ui.add_sized(egui::vec2(120.0, 30.0), egui::Button::new(RichText::new("Save Image 💾"))).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(".wav File", &["wav"])
                            .save_file()
                        {
                            let wave_spec = hound::WavSpec {
                                channels: 1,
                                sample_rate: 44100,
                                bits_per_sample: 16,
                                sample_format: hound::SampleFormat::Int,
                            };

                            let mut writer = hound::WavWriter::create(path, wave_spec).unwrap();
                            for p in sound_samples {
                                let _ = writer.write_sample(*p);
                            }
                            let _ = writer.finalize();
                        }
                    }
                }


                
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center),|ui| {
                ui.heading(RichText::new(format!("{}", *self.program_status.lock().unwrap())).size(24.0));
            }) 
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                if let Some(texture) = &self.main_texture_handle {
                    ui.image((texture.id(), egui::vec2(800.0, 600.0)));
                }
            })
        });
    }
}

fn main() {
    let icon_data = {
        let bytes = include_bytes!("app_icon.png");
        let img = image::load_from_memory(bytes).expect("failed to load").to_rgba8();
        egui::IconData {rgba:img.into_vec(), width:256, height:256}
    };

    let viewport = egui::ViewportBuilder::default()
    .with_inner_size(egui::vec2(1200.0, 800.0))
    .with_icon(icon_data);

    let options = eframe::NativeOptions {
        viewport,..Default::default()
    };
    let _ = eframe::run_native("SSTV_2", options, Box::new(|_cc| Ok(Box::new(Globals::default()))));
}

