use egui::ColorImage;
use crate::Mode;

enum State {
    Ready,
    SyncWait,
    SyncStart,
    Done,
}

fn tolerance(value: f32, target: f32, tol: f32) -> bool {
    return f32::abs(value-target) <= tol
}

pub fn decode_image(frequency_samples: Vec<f32>, mode: Mode) -> ColorImage {

    let mut width: usize = 320;
    let mut num_lines: usize =  256;
    let n;

    match mode {
        Mode::RAW => {
            n = 1;
            width = 640;
        }
        Mode::MartinM1 | Mode::MartinM2 => {
            n = 3;
        }
        Mode::PD120 | Mode::PD180 => {
            n = 4;
            num_lines = 496;
            width = 640;
        }
    }
    width = width*n;

    let mut state = State::Ready;
    let mut row_buf: Vec<f32> = Vec::new();
    let mut lines: Vec<Vec<f32>> = Vec::new();
    let max_blank = 50;
    let mut blank = max_blank;

    for (i, &f) in frequency_samples.iter().enumerate() {

        if i == frequency_samples.len() - 1 {
            state = State::Done;
        }
        match state {
            State::Ready => {
                if tolerance(f, 1900.0, 5.0) {state = State::SyncWait}
            }
            State::SyncWait => {
                if tolerance(f, 1200.0, 20.0) {
                    if row_buf.len() >= width {
                        state = State::SyncStart;
                        blank = max_blank; 
                        lines.push(row_buf.clone());
                        if lines.len() > num_lines {
                            state = State::Done
                        }
                    }
                    row_buf.clear();
                } else {
                    let l = f32::round(f32::abs((255.0)*(f-1500.0)/(2300.0-1500.0)));
                    row_buf.push(l);
                }
            }
            State::SyncStart => {
                if blank > 0 {
                    blank -= 1;
                } else {
                    if tolerance(f, 1200.0, 300.0) == false {state = State::SyncWait}
                }
                
            }
            State::Done => {

                let mut data_grid: Vec<u8> = Vec::new();

                println!("Decoding Complete, {} Lines Found", lines.len());

                for line in lines.clone() {
                    if line.len() >= width {
                        for i in 0..width {
                            let line_len = line.len();
                            let line_slice = &line[((i*(line_len-1))/width)..((((i+1)*(line_len-1))/width))];
                            let mut l: f32 = 0.0;
                            if line_slice.len() > 0 {
                                for v in line_slice {
                                  l += v
                                }
                                l = l/(line_slice.len() as f32);
                            }
                            let y = f32::round(l) as u8;
                            data_grid.push(y);
                        }
                    } else {
                        for i in 0..width {
                            let y = line[(i/width)*(line.len()-1)] as u8;
                            data_grid.push(y);
                        }
                    }
                }
                
                let mut pixels: Vec<egui::Color32> = Vec::new();

                match mode {
                    Mode::RAW => {
                        for val in data_grid {
                            pixels.push(egui::Color32::from_rgb(val,val,val));
                        }
                    }
                    Mode::MartinM1 | Mode::MartinM2 => {
                        for (i, &val) in data_grid.iter().enumerate() {
                            if i % (width) < (width/3) {
                                pixels.push(egui::Color32::from_rgb( data_grid[i+(2*width/3)],val, data_grid[i+(width/3)]))
                            }
                        }   
                    }
                    Mode::PD120 | Mode::PD180 => {
                        for (i, &val) in data_grid.iter().enumerate() {
                            if i % (width) < (width/4) {
                                let cr = data_grid[i+(width/4)] as f32;
                                let cb = data_grid[i+(width/2)] as f32;
                                let r = f32::round(val as f32 + (1.402*(cr - 128.0))).clamp(0.0, 255.0) as u8;
                                let g = f32::round(val as f32 - (0.344*(cb - 128.0)) - (0.714*(cr - 128.0))).clamp(0.0, 255.0) as u8;
                                let b = f32::round(val as f32 + (1.772*(cb - 128.0))).clamp(0.0, 255.0) as u8;
                                pixels.push(egui::Color32::from_rgb(r,g,b))
                            }
                        }
                    }
                }

                let new_image = egui::ColorImage {
                    size: [width/n as usize, lines.len() as usize],
                    source_size: egui::Vec2 { x: (width/n) as f32, y: (lines.len()) as f32 },
                    pixels,
                };

                return new_image
            }
        }


    }

    return ColorImage::example();
}