mod ring_buffer;
mod adsr;
use ring_buffer::RingBuffer;
mod sampler_voice;
mod sampler_engine;
use hound::{WavReader, WavSpec, SampleFormat};
use std::io::BufReader;
use std::fs::File;
use sampler_voice::SamplerVoice;
use sampler_engine::SamplerEngine;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    /* let mut reader = hound::WavReader::open(&args[1]).unwrap();
    let wav_size = reader.len();
    let channels = reader.spec().channels as usize;
    let mut wav_as_ring = RingBuffer::<f32>::new(wav_size as usize);
    fill_buffer(&mut wav_as_ring, &mut reader); */
    let base_note = 60;
    //let mut sampVoice = SampleVoice::new( channels,base_note);
    /* let mut voices: Vec<SamplerVoice> = (0..5)
    .map(|_| SamplerVoice::new(channels, base_note))
    .collect(); */
    let new_sample_rate = 48000.0;
    let mut engine = SamplerEngine::new(new_sample_rate,2);
    engine.add_to_paths_and_load(&args[1]);
    engine.set_warp_base(base_note);
    engine.assign_file_to_midi(&args[1], 60);
    engine.assign_file_to_midi(&args[2], 51);
    engine.assign_file_to_midi(&args[3], 48);

    let newSpec = hound::WavSpec {
        channels: 2,
        sample_rate: new_sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    engine.set_mode(sampler_engine::SamplerMode::Assign);
    engine.set_adsr(0.1, 0.1, 0.1, 0.4);

   /*  for (i,voice) in voices.iter_mut().enumerate(){
        voice.set_note(base_note + (i as u8 * 5));
    } */
    //sampVoice.set_note(midi_note);
    let mut writer = hound::WavWriter::create(&args[4], newSpec).unwrap();
    for i in (0..48000*10){
        let mut outSample = 0.0;
        /* for voice in voices.iter_mut(){
            outSample += voice.processWarp(&mut wav_as_ring);
        } */
        if i == 1000{
            engine.note_on(30,1.0);
        }
        if i == 48000-30000 {
            engine.note_on(60, 1.0)
        }
        if i == 58000-30000{
            engine.note_on(63, 1.0);
        }
        if i == 70000-30000{
            engine.note_on(48, 1.0);
            engine.note_on(51, 1.0);
        }
        if i == 60000-30000{
            engine.note_on(59,1.0);
        }
        if i == 100000{
            engine.note_off(60);
            engine.note_off(63);
            engine.note_off(48);
            engine.note_off(51);
            engine.note_off(59);
        }
        outSample = engine.process();
        //let outSample = sampVoice.process(&mut wav_as_ring);
        writer.write_sample(outSample);
    }
    writer.finalize().unwrap();
    //println!("This is your sample {} semitones apart", midi_note as i8 -base_note as i8);
    println!("This is your sample... but with chords now!!");
}

