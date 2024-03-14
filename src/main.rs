mod ring_buffer;
use ring_buffer::RingBuffer;
mod sample_voice;
use hound::{WavReader, WavSpec, SampleFormat};
use std::io::BufReader;
use std::fs::File;
use sample_voice::SampleVoice;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut reader = hound::WavReader::open(&args[1]).unwrap();
    let wav_size = reader.len();
    let channels = reader.spec().channels as usize;
    let mut wav_as_ring = RingBuffer::<f32>::new(wav_size as usize);
    fill_buffer(&mut wav_as_ring, &mut reader);
    let base_note = 60;
    //let mut sampVoice = SampleVoice::new( channels,base_note);
    let mut voices: Vec<SampleVoice> = (0..5)
    .map(|_| SampleVoice::new(channels, base_note))
    .collect();

    let newSpec = hound::WavSpec {
        channels: reader.spec().channels,
        sample_rate: reader.spec().sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let midi_note: u8 = match args[3].parse() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Invalid Offset: {}", args[3]);
            return;
        }
    };

    for (i,voice) in voices.iter_mut().enumerate(){
        voice.set_note(base_note + (i as u8 * 5));
    }
    //sampVoice.set_note(midi_note);
    
    let mut writer = hound::WavWriter::create(&args[2], newSpec).unwrap();
    for i in (0..wav_size){
        let mut outSample = 0.0;
        for voice in voices.iter_mut(){
            outSample += voice.process(&mut wav_as_ring);
        }
        //let outSample = sampVoice.process(&mut wav_as_ring);
        writer.write_sample(outSample);
    }
    writer.finalize().unwrap();
    //println!("This is your sample {} semitones apart", midi_note as i8 -base_note as i8);
    println!("This is your sample... but with chords now!!");
}


/// Fills the internal buffers with the samples from the wav file
/// 
/// Takes into account the number of channels, bits per sample, and float vs int values.  
fn fill_buffer(buffer: &mut RingBuffer<f32>, reader: &mut WavReader<BufReader<File>>) {
    let sample_format = reader.spec().sample_format;
    let num_channels = reader.spec().channels as usize;
    let sample_rate = reader.spec().sample_rate as f32;
    let length = reader.len();
    // Determine the conversion factor based on sample format
    let conversion_factor = match sample_format {
        SampleFormat::Float => 1.0, // No conversion needed
        SampleFormat::Int => {
            match reader.spec().bits_per_sample {
                8 => 1.0 / (i8::MAX as f32),
                16 => 1.0 / (i16::MAX as f32),
                24 => 1.0 / (8388608 as f32), 
                _ => panic!("Unsupported bit depth"),
            }
        }
    };
    match sample_format{
        SampleFormat::Float => {
            let mut samples = reader.samples::<f32>();
            for _ in 0..(length) {
                if let Some(sample) = samples.next() {
                    if let Ok(sample_value) = sample {
                        let sample_float = sample_value * conversion_factor;
                         buffer.push(sample_float);
                    }
                }
            }
        }, 
        SampleFormat::Int => {
            let mut samples = reader.samples::<i32>();
            for _ in 0..(length) {
                
                if let Some(sample) = samples.next() {
                    if let Ok(sample_value) = sample {
                        let sample_float = (sample_value as f32) * conversion_factor;
                        buffer.push(sample_float);
                    }
                }
            }
        }
    }
}

