use num_complex::{Complex, c32};

pub fn fft(time_samples: Vec<Complex<f32>>) -> Vec<Complex<f32>> {

    let n = time_samples.len() as f32;

    let log_n = f32::log2(n);
    let mut new_samples: Vec<Complex<f32>>;

    if log_n % 1.0 == 0.0 {
        new_samples = time_samples;
    } else {
        let ceil_n: usize = 2.0_f32.powf(f32::ceil(log_n)) as usize;

        new_samples = Vec::with_capacity(ceil_n);
        for i in 0..n as usize {
            new_samples.push(time_samples[i]);
        }
        for _i in n as usize ..(ceil_n) {
            new_samples.push(c32(0.0, 0.0));
        }
    }
    return subdivide(new_samples)
}

fn subdivide(samples: Vec<Complex<f32>>) -> Vec<Complex<f32>> {

    let n = samples.len() as u32;
    if n <= 1 {
        return samples
    }

    let m: u32 = n/2;

    let mut even: Vec<Complex<f32>> = Vec::with_capacity(m as usize);
    let mut odd: Vec<Complex<f32>> = Vec::with_capacity(m as usize);

    for i in 0..m {
        even.push(samples[(2*i) as usize]);
        odd.push(samples[((2*i)+1) as usize]);
    }

    let f_even: Vec<Complex<f32>> = subdivide(even);
    let f_odd: Vec<Complex<f32>> = subdivide(odd);

    let mut f_bins_low: Vec<Complex<f32>> = Vec::with_capacity(m as usize);
    let mut f_bins_high: Vec<Complex<f32>> = Vec::with_capacity(m as usize);

    for k in 0..m as usize {
        let pi: f32 = std::f32::consts::PI;
        let twiddle: Complex<f32> = f_odd[k] * Complex::exp(c32(0.0,-2.0*pi*(k as f32)/(n as f32)));      
        f_bins_low.push(f_even[k] + twiddle);
        f_bins_high.push(f_even[k] - twiddle);
    }

    let freq_bins: Vec<Complex<f32>> =[f_bins_low,f_bins_high].concat();

    return freq_bins
}

pub fn ifft(samples: Vec<Complex<f32>>) -> Vec<Complex<f32>> {

    let n = samples.len();

    if n <= 1 {
        return samples
    }

    let mut conj: Vec<Complex<f32>> = Vec::with_capacity(n);

    for x in samples {
        conj.push(x.conj());
    }

    let fft_conj = fft(conj);

    let mut res_vec: Vec<Complex<f32>> = Vec::with_capacity(n);

    for z in fft_conj {
        res_vec.push(z.conj()/(n as f32));
    }

    return res_vec

}

pub fn hilbert(samples: Vec<Complex<f32>>, f_samp: f32, f_low: f32, f_high: f32) -> Vec<Complex<f32>> {
    let hfft = fft(samples);
    let hfft_len = hfft.len();

    let mut hilbert: Vec<Complex<f32>> = Vec::with_capacity(hfft_len);

    for i in 0..hfft_len {

        let z = hfft[i];
        let mut h: f32 = 1.0;

        if i > 0 && i < (hfft_len/2) {
            h = 2.0;
        } else if i > (hfft_len/2) {
            h = 0.0;
        }

        hilbert.push(c32(z.re * h, z.im * h));
    }

    let hifft = ifft(bp_filter(hilbert, f_samp, f_low, f_high));

    return  hifft;
}

fn bp_filter(samples: Vec<Complex<f32>>, f_samp: f32, f_low: f32, f_high: f32) -> Vec<Complex<f32>> {
    let samp_len = samples.len() as f32; 
    let mut filtered_samps: Vec<Complex<f32>> = Vec::with_capacity(samples.capacity());
    for (ix, bin) in samples.iter().enumerate() {
        let f = if ix as f32 <= samp_len/2.0 {
            (ix as f32) * f_samp / samp_len
        } else {
            ((ix as f32) - samp_len) * f_samp / samp_len
        };

        if f <= f_high && f >= f_low {
            filtered_samps.push(*bin);
        } else {
            filtered_samps.push(Complex::new(0.0, 0.0));
        }

    }

    return filtered_samps
}