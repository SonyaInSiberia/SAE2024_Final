mod ring_buffer;
mod adsr;
use ring_buffer::RingBuffer;
mod sampler_voice;
mod sampler_engine;
mod crossfade;
use hound::{WavReader, WavSpec, SampleFormat};
use std::io::BufReader;
use std::fs::File;
use sampler_voice::SustainModes;
use sampler_engine::SamplerEngine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let base_note = 60;
    let new_sample_rate = 48000.0;
    let mut engine = SamplerEngine::new(new_sample_rate,2);
    engine.add_to_paths_and_load(&args[1]);
    engine.set_warp_base(base_note);
    engine.assign_file_to_midi(&args[2], 60);
    engine.assign_file_to_midi(&args[1], 51);
    engine.assign_file_to_midi(&args[3], 48);
    

    let newSpec = hound::WavSpec {
        channels: 2,
        sample_rate: new_sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    engine.set_mode(sampler_engine::SamplerMode::Warp);
    engine.set_sus_looping_assign(SustainModes::LoopBounce, 51);
    engine.set_sus_looping_warp(SustainModes::LoopBounce);
    engine.set_fade_time_warp(0.01);
    engine.set_adsr_warp(0.0, 0.0, 1.0, 0.4);
    engine.set_adsr_assign(0.1, 0.0, 1.0, 0.2, 60);

    engine.set_adsr_assign(0.10, 0.0, 1.0, 0.3, 51);
    engine.set_points_assign(0.0, 100.0, 51);
    engine.set_sus_points_assign(10.0, 20.0, 51);
    engine.set_fade_time_assign(0.01, 51);

    engine.set_points_warp(0.0, 100.0);
    engine.set_sus_points_warp(14.0, 16.0);

    let mut writer = hound::WavWriter::create(&args[4], newSpec).unwrap();

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("failed to find default input device");
    let config = device.default_input_config().unwrap();
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    let buffer_size: usize = 512;

    // Create a ring buffer to hold audio samples
    let ring_buffer = RingBuffer::<f32>::new(buffer_size * channels);
    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };
    let stream = device.build_input_stream(
        &config.into(),
        move |data, _: &_| input_callback(data),
        err_fn,
        None,
    ).unwrap();
    stream.play().unwrap();
    /* loop{
        
    } */

    for i in (0..480000){
        let mut outSample = 0.0;
        /* if i == 1000{
            engine.note_on(30,1.0);
        }
        if i == 48000-30000 {
            engine.note_on(60, 1.0)
        }
        if i == 58000-30000{
            engine.note_on(63, 1.0);
        } */
        if i == 70000-60000{//30000
            //engine.note_on(49, 1.0);
            engine.note_on(51, 1.0);
        }
        /* if i == 60000-30000{
            engine.note_on(59,1.0);
        }
        if i == 100000{
            engine.note_off(60);
            engine.note_off(63);
            engine.note_off(48);
            engine.note_off(59);
        } */
        if i == 400000{
            engine.note_off(51);
        }
        outSample = engine.process();
        writer.write_sample(outSample);
    }
    writer.finalize().unwrap();
    //println!("This is your sample {} semitones apart", midi_note as i8 -base_note as i8);
    println!("This is your sample... but with chords now!!");
}

fn input_callback(input: &[f32]){
    for &sample in input.iter(){

    }
}
