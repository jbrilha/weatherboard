use core::f32::consts::PI;

use defmt::info;
use embassy_stm32::{peripherals, sai::Sai};

use embassy_time::Timer;
use micromath::F32Ext;

#[embassy_executor::task]
pub async fn freq_glide_task(mut sai_a: Sai<'static, peripherals::SAI1, u32>) {
    info!("smooth frequency glide");

    // let amp = 0.3f32;

    loop {
        // glide through frequencies in small steps; recompute phase fresh each block
        for step in 0..=60 {
            let amp = 0.1 + (step as f32 / 60.0) * 0.8; // volume ramps with the sweep
            // 0..30 rising, 30..60 falling -> siren
            let t = if step <= 30 { step } else { 60 - step } as f32 / 30.0; // 0..1
            let freq = (200.0 + t * 1800.0) as usize; // 200..2000 Hz
            let frames = 48_000 / freq.max(1); // samples per cycle

            let mut tone = [0u32; 256 * 2]; // max 48000/200 = 240
            for n in 0..frames {
                let phase = (n as f32) / (frames as f32) * core::f32::consts::TAU;
                let v16 = (phase.sin() * amp * (i16::MAX as f32)) as i16;
                let v = ((v16 as i32) << 16) as u32;
                tone[2 * n] = v;
                tone[2 * n + 1] = v;
            }

            // hold each freq briefly (~50ms) so the glide is smooth but quick
            let repeats = 2_400 / frames; // ~50ms
            for _ in 0..repeats {
                sai_a.write(&tone[..frames * 2]).await.ok();
            }

            Timer::after_millis(1000).await;
        }
    }
}

#[embassy_executor::task]
pub async fn tone_task(mut sai_a: Sai<'static, peripherals::SAI1, u32>) {
    info!("step tone, varying pitch");
    let amp = 0.3f32;

    // Frequencies that divide 48000 evenly -> clean loops, same style as beep
    const FREQS: [usize; 6] = [250, 500, 1000, 1500, 2000, 1000];

    loop {
        for &freq in FREQS.iter() {
            let frames = 48_000 / freq; // samples per cycle
            // build one cycle, exactly like the beep test does
            let mut tone = [0u32; 192 * 2]; // max size (48000/250 = 192)
            for n in 0..frames {
                let phase = (n as f32) / (frames as f32) * core::f32::consts::TAU;
                let v16 = (phase.sin() * amp * (i16::MAX as f32)) as i16;
                let v = ((v16 as i32) << 16) as u32;
                tone[2 * n] = v;
                tone[2 * n + 1] = v;
            }
            info!("playing {} Hz", freq);
            // play this freq for ~0.5s: each cycle is frames samples = frames/48000 s
            // number of repeats for 0.5s:
            let repeats = 24_000 / frames; // 0.5s worth
            for _ in 0..repeats {
                // write only the first `frames` stereo samples
                sai_a.write(&tone[..frames * 2]).await.ok();
            }
        }
    }
}

#[embassy_executor::task]
pub async fn beep_task(mut sai_a: Sai<'static, peripherals::SAI1, u32>) {
    info!("beep beep beep");
    const FRAMES: usize = 48; // 1 kHz
    const TONE_PERIODS: usize = 200; // 200 cycles = 200 ms of 1kHz @ 48k
    const SIL_PERIODS: usize = 800; // 800 "cycles" worth = 800 ms silence

    // Tone block: 200 ms
    let mut tone = [0u32; FRAMES * 2];
    let amp = 0.3f32;
    for n in 0..FRAMES {
        let phase = (n as f32) / (FRAMES as f32) * core::f32::consts::TAU;
        let v16 = (phase.sin() * amp * (i16::MAX as f32)) as i16;
        let v = ((v16 as i32) << 16) as u32;
        tone[2 * n] = v;
        tone[2 * n + 1] = v;
    }

    let silence = [0u32; FRAMES * 2];

    loop {
        // 200 ms of tone = 200 periods
        for _ in 0..TONE_PERIODS {
            sai_a.write(&tone).await.ok();
        }
        // 800 ms of silence = 800 periods
        for _ in 0..SIL_PERIODS {
            sai_a.write(&silence).await.ok();
        }
    }
}

#[embassy_executor::task]
pub async fn square_wave_task(mut sai_a: Sai<'static, peripherals::SAI1, u32>) {
    info!("square wave");

    // Generate test signal: 1 kHz square wave at 48 kHz sample rate
    // Signal period: 48_000 / 1_000 = 48 samples per period
    let sample_rate = 48_000;
    let test_freq = 1_000;
    let period_samples = sample_rate / test_freq;
    let half_period = period_samples / 2;

    let mut test_data = [0u32; 128]; // 128 samples = 256 bytes (stereo)
    for i in 0..test_data.len() {
        let sample = if (i % period_samples) < half_period {
            0x7FFFFFFF // Maximum amplitude (positive)
        } else {
            0x80000000 // Minimum amplitude (negative)
        };
        // Stereo: same signal on both channels
        test_data[i] = sample;
    }

    info!("Test signal buffer prepared, starting playback loop");

    loop {
        if let Err(e) = sai_a.write(&test_data).await {
            info!("SAI write error: {:?}", e);
        }
    }
}

#[embassy_executor::task]
pub async fn sine_wave_task(mut sai_a: Sai<'static, peripherals::SAI1, u32>) {
    info!("sine wave");

    // Generate test signal: 1 kHz sine wave at 48 kHz sample rate
    // Signal period: 48_000 / 1_000 = 48 samples per period
    let sample_rate = 48_000.0;
    let test_freq = 1_000.0;
    let period_samples = (sample_rate / test_freq) as usize;

    let mut test_data = [0u32; 128];

    for i in 0..test_data.len() {
        // Phase angle for this sample
        let phase = 2.0 * PI * (i % period_samples) as f32 / period_samples as f32;

        // Generate sine sample in range [-1.0, 1.0]
        let sine = phase.sin();

        // Scale to signed 32-bit PCM range
        let sample_i32 = (sine * i32::MAX as f32) as i32;

        // Reinterpret as u32 for SAI peripheral
        test_data[i] = sample_i32 as u32;
    }

    info!("Sine wave buffer prepared, starting playback loop");

    loop {
        if let Err(e) = sai_a.write(&test_data).await {
            info!("SAI write error: {:?}", e);
        }
    }
}
