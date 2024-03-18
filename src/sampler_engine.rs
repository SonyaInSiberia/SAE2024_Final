use crate::{sampler_voice,ring_buffer,adsr};
use sampler_voice::SamplerVoice;
use ring_buffer::RingBuffer;
use std::collections::HashMap;
use hound::{WavReader, WavSpec, SampleFormat};
use adsr::AdsrState;

pub struct SamplerEngine{
    num_voices: u8,
    file_paths: HashMap<String,String>,
    files_to_assign: Vec<RingBuffer<f32>>,
    warp_buffer: RingBuffer<f32>,
    sampler_mode: SamplerMode,
    voices: Vec<SamplerVoice>,
    sample_rate: f32,
    num_channels: usize,
}
pub enum SamplerMode{
    Warp, // For when you just load one sample and want it to be pitch warped
    Assign, // For when you load multiple samples and assign them to midi notes
    Sfz, // For when you load an sfz file
}

impl SamplerEngine{
    pub fn new(sample_rate_: f32, num_channels_: usize, num_voices_: u8, sampler_mode_: SamplerMode) -> Self{
        
        let files = vec![RingBuffer::<f32>::new(1);1];
        let mut buff = RingBuffer::<f32>::new(1);
        let voices_ = vec![SamplerVoice::new(num_channels_,64);6];

        let engine = SamplerEngine{
            num_voices: num_voices_,
            file_paths: HashMap::with_capacity(30),
            files_to_assign: files,
            warp_buffer: buff,
            sampler_mode: sampler_mode_,
            voices: voices_,
            sample_rate: sample_rate_,
            num_channels: num_channels_,
        };
        engine
    }
    pub fn process(&mut self)->f32{
        let mut out_samp = 0.0;
        match self.sampler_mode{
            SamplerMode::Warp =>{

                for voice in self.voices.iter_mut(){
                    out_samp += voice.processWarp(&mut self.warp_buffer);
                }
            },
            SamplerMode::Assign =>{
                todo!("Actually implent this lol !");

                for voice in self.voices.iter_mut(){
                    out_samp += voice.processAssign(&mut self.warp_buffer);
                }

            },
            SamplerMode::Sfz =>{
                todo!("Actually implent this lol !");
                out_samp = 0.0;
            }
        }
        out_samp
    }
    pub fn add_to_paths_and_load(&mut self, file_path: &str, keyword: &str){
        fill_buffer(&mut self.warp_buffer, file_path);
        self.file_paths.insert(keyword.to_string(),file_path.to_string());
    }
    pub fn add_file_to_paths(&mut self, file_path: &str, keyword: &str){
        self.file_paths.insert(keyword.to_string(),file_path.to_string());
    }

    pub fn load_file_from_path(&mut self, file_path: &str){
        fill_buffer(&mut self.warp_buffer, file_path);
    }

    pub fn load_file_from_key(&mut self, keyword: &str){
        let file_path = self.file_paths.get(keyword);
        fill_buffer(&mut self.warp_buffer, file_path.unwrap());
    }

    pub fn note_on(&mut self, note: u8, velocity: u8){
        let voice_id = self.get_voice_id();
        self.voices[voice_id].note_on(note, velocity);
    }

    pub fn note_off(mut self, note: u8){
        for voice in self.voices.iter_mut(){
            if voice.midi_note == note{
                voice.note_off();
                break;
            }
        }
    }

    fn get_voice_id(&mut self)-> usize{
        for (voice_id, voice) in self.voices.iter_mut().enumerate() {
            if !voice.is_active() {
                return voice_id;
            }
        }
        if let Some((quietest_voice_id, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.adsr.state == AdsrState::Release)
            .min_by(|(_, voice_a), (_, voice_b)| {
                f32::total_cmp(
                    &voice_a.adsr.envelope_value,
                    &voice_b.adsr.envelope_value,
                )
            })
        {
            return quietest_voice_id;
        }

        let (quietest_voice_id, _) = self
            .voices
            .iter()
            .enumerate()
            .min_by(|(_, voice_a), (_, voice_b)| {
                f32::total_cmp(
                    &voice_a.adsr.envelope_value,
                    &voice_b.adsr.envelope_value,
                )
            })
            .unwrap();

        quietest_voice_id
    }

}

fn fill_buffer(buffer: &mut RingBuffer<f32>, path: &str) {
    // TODO: Add sample rate multiplier
    let mut reader = hound::WavReader::open(path).unwrap();
    let sample_format = reader.spec().sample_format;
    let num_channels = reader.spec().channels as usize;
    let sample_rate = reader.spec().sample_rate as f32;
    let length = reader.len();
    buffer.resize(length as usize, 0.0);
    buffer.set_write_index(0);
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