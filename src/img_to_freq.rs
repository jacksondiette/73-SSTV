use egui::ColorImage;
use crate::Mode;

pub fn encode(image_data: ColorImage, mode: Mode) -> Vec<i16> {
    let mut freq_vec: Vec<f32> = Vec::new();
    let vis: u8;

    match mode {
        Mode::MartinM1 => {
            vis = 44;
        }
        Mode::MartinM2 => {
            vis = 40;
        }
        Mode::RAW | Mode::PD120 | Mode::PD180 => {
            vis = 0;
        }
    }

    let f_samp: f32 = 44100.0;

    let mut add_freq = |freq: f32, time: f32| {
        for _i in 0..f32::floor(f_samp*time) as i32 {
            freq_vec.push(freq);
        }
    };

    // SSTV Structure

    // Leader Tone
    add_freq(1900.0, 0.3);

    // Break
    add_freq(1200.0, 0.01);

    // Leader Tone
    add_freq(1900.0, 0.3);

    // Break
    add_freq(1200.0, 0.03);

    // VIS Code
    let mut parity: u8 = 0;
    for i in 0..7 {
        if (vis & (1 << i)) != 0 {
            add_freq(1300.0, 0.03);
            print!("1");
            parity += 1;
        } else {
            add_freq(1100.0, 0.03);
            print!("0")
        }
    }


    // Parity Bit
    if parity % 2 == 0 {
        add_freq(1300.0, 0.03);
    } else {
        add_freq(1100.0, 0.03);
    }

    // Stop Bit
    add_freq(1200.0, 0.03);

    let img_dim: [usize; 2] = [image_data.width(), image_data.height()];
    let old_pix_vec: Vec<egui::Color32> = image_data.pixels;

    match mode {
        Mode::MartinM1 | Mode::MartinM2 => {
            let num_lines: usize = 256;
            let line_len: usize = 320;

            let pix_vec = resize(old_pix_vec, img_dim[0] as u32, line_len as u32, img_dim[1] as u32,num_lines as u32);

            let t_pix;
            
            if mode == Mode::MartinM1 {
                t_pix = (146.432/1000.0) / (line_len as f32);
            } else {
                t_pix = (73.216/1000.0) / (line_len as f32);
            }

            for line_num in 0..num_lines-1 {

                add_freq(1200.0, 4.862/1000.0);
                add_freq(1500.0, 0.572/1000.0);

                let line_buffer: Vec<egui::Color32> = pix_vec[line_num*line_len..(line_num+1)*line_len].to_vec();
                let mut yuv_buffer: Vec<f32> = vec![0.0; line_len*3];

                for (i, pix) in line_buffer.iter().enumerate() {
                    let r = pix.r() as f32 / 255.0;
                    let g = pix.g() as f32 / 255.0;
                    let b = pix.b() as f32 / 255.0;
    
                    yuv_buffer[i] = r;
                    yuv_buffer[i + line_len] = g;
                    yuv_buffer[i + (line_len * 2)] = b;
                }

                for (i, f) in yuv_buffer.iter().enumerate() {
                    add_freq(*f * 800.0 + 1500.0, t_pix);
                    if (i+1) % line_len == 0 {
                        add_freq(1500.0, 0.572/1000.0);
                    }
                }
            }
        }
        Mode::RAW | Mode::PD120 | Mode::PD180 => {
            println!("How did you get here?");
        }
    }

    let mut phase: f32 = 0.0;
    let mut phase_vec: Vec<i16> = Vec::with_capacity(freq_vec.capacity());

    const TAU: f32 = 2.0 * std::f32::consts::PI;

    for freq in freq_vec {
        phase += freq * TAU / f_samp;
        if phase >= TAU {
            phase -= TAU;
        }
        let sample = (phase.sin() * i16::MAX as f32) as i16;
        phase_vec.push(sample);
    }

    return phase_vec;

}

pub fn resize(pixel_vec: Vec<egui::Color32>, old_width: u32, new_width: u32, old_height: u32, new_height: u32) -> Vec<egui::Color32> {

    let mut img = image::RgbImage::new(old_width as u32, old_height as u32);
    for y in 0..old_height {
        for x in 0..old_width {
            let idx: usize = (y * old_width + x) as usize;
            img.put_pixel(x as u32, y as u32, image::Rgb([pixel_vec[idx][0], pixel_vec[idx][1], pixel_vec[idx][2]]));
        }
    };

    let resized_image = image::imageops::resize(&img, new_width, new_height, image::imageops::FilterType::Gaussian);

    let new_vec: Vec<egui::Color32> = resized_image.pixels().map(|p| egui::Color32::from_rgb(p[0], p[1], p[2])).collect();

    return new_vec
}