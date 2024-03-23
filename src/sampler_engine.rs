use crate::{sampler_voice,ring_buffer,adsr};
use sampler_voice::SamplerVoice;
use ring_buffer::RingBuffer;
use std::collections::HashMap;
use hound::{WavReader, WavSpec, SampleFormat};
use adsr::AdsrState;

pub struct SamplerEngine{
    num_voices: u8,
    sound_bank: HashMap<u8,(String,f32,RingBuffer<f32>)>,
    file_names: Vec<String>,
    warp_buffer: RingBuffer<f32>,
    sampler_mode: SamplerMode,
    warp_voices: Vec<SamplerVoice>,
    assign_voices: Vec<SamplerVoice>,
    sample_rate: f32,
    num_channels: usize,
    warp_sr_scalar: f32,
}
#[derive(PartialEq)]
pub enum SamplerMode{
    Warp, // For when you just load one sample and want it to be pitch warped
    Assign, // For when you load multiple samples and assign them to midi notes
    Sfz, // For when you load an sfz file
}

impl SamplerEngine{
    pub fn new(sample_rate_: f32, num_channels_: usize) -> Self{
        
        let files = vec!["".to_string();100];
        let buff = RingBuffer::<f32>::new(1);
        let voices_ = vec![SamplerVoice::new(num_channels_,64);6];
        let other_voices = vec![SamplerVoice::new(num_channels_,64);1];

        let mut engine = SamplerEngine{
            num_voices: 6,
            sound_bank: HashMap::with_capacity(30),
            file_names: files,
            warp_buffer: buff,
            sampler_mode: SamplerMode::Warp,
            warp_voices: voices_,
            assign_voices: other_voices,
            sample_rate: sample_rate_,
            num_channels: num_channels_,
            warp_sr_scalar: sample_rate_,
        };
        engine.file_names.clear();
        engine.assign_voices.clear();
        engine
    }
    pub fn process(&mut self)->f32{
        let mut out_samp = 0.0;
        match self.sampler_mode{
            SamplerMode::Warp =>{
                for voice in self.warp_voices.iter_mut(){
                    out_samp += voice.processWarp(&mut self.warp_buffer, 
                                                self.warp_sr_scalar);
                }
            },
            SamplerMode::Assign =>{
                for voice in self.assign_voices.iter_mut(){
                    let (file_name, sr_scalar,buff) = 
                                                self.sound_bank.get_mut(&voice.base_midi).unwrap();
                    out_samp += voice.processAssign(buff,*sr_scalar);
                }
            },
            SamplerMode::Sfz =>{
                todo!("Actually implent this lol !");
                out_samp = 0.0;
            }
        }
        out_samp
    }
    ///Add a file to the paths of files saved in the file names
    /// and load file into the warp buffer.
    pub fn add_to_paths_and_load(&mut self, file_path: &str){
        self.warp_sr_scalar =  fill_warp_buffer(&mut self.warp_buffer, file_path)/
                                self.sample_rate;
        self.file_names.push(file_path.to_string());
    }
    ///Add a file to the paths of files saved in the file names.
    pub fn add_file_to_paths(&mut self, file_path: &str){
        self.file_names.push(file_path.to_string());
    }
    ///Load file from path into the warp buffer without loading 
    /// into the file names.
    pub fn load_file_from_path(&mut self, file_path: &str){
        self.warp_sr_scalar =  fill_warp_buffer(&mut self.warp_buffer, file_path)/
                                self.sample_rate;
    }
    /// Assigns an audio file to a midi note for the sound bank. (Assign mode)
    /// 
    /// Will add file to paths if not already there
    pub fn assign_file_to_midi(&mut self, file_path: &str, note: u8){
        if !self.file_names.contains(&file_path.to_string()){
            self.add_file_to_paths(file_path);
        }
        let (buff,sr) = create_buffer(file_path);
        let sr_scalar = sr / self.sample_rate;
        self.sound_bank.insert(note,(file_path.to_string(),sr_scalar,buff));
        self.assign_voices.push(SamplerVoice::new(self.num_channels,note)); 
    }

    /// Triggers a "note on" message and allocates a voice, 
    ///  stealing if necessary
    pub fn note_on(&mut self, note: u8, velocity: f32){
        match self.sampler_mode {
            SamplerMode::Warp =>{
                let voice_id = self.get_voice_id();
                self.warp_voices[voice_id].note_on(note, velocity);
            },
            SamplerMode::Assign =>{
                for voice in self.assign_voices.iter_mut(){
                    if voice.base_midi == note{
                        voice.note_on(note, velocity);
                        break;
                    }
                }
                
            },
            SamplerMode::Sfz =>{

            }
        }
    }
    /// Triggers a note off message
    pub fn note_off(&mut self, note: u8){
        match self.sampler_mode {
            SamplerMode::Warp =>{
                for voice in self.warp_voices.iter_mut(){
                    if voice.midi_note == note{
                        voice.note_off();
                        break;
                    }
                }
            },
            SamplerMode::Assign =>{
                for voice in self.assign_voices.iter_mut(){
                    if voice.base_midi == note{
                        voice.note_off();
                        break;
                    }
                }
                
            },
            SamplerMode::Sfz =>{

            }
        }
    }
    /// Sets the attack, decay, sustain, and release for all the warp sample voices
    pub fn set_adsr(&mut self, attack_: f32, decay_: f32, sustain_: f32, release_: f32){
        for voice in self.warp_voices.iter_mut(){
            voice.set_adsr(attack_, decay_, sustain_, release_);
        }
    }
    /// Sets the max number of voices in the sampler
    pub fn set_num_voices(&mut self, num_voices: u8){
        self.warp_voices.resize(num_voices as usize, 
            SamplerVoice::new(self.num_channels,64));
    }
    /// Sets the sampler mode (Warp, Assign, Sfz)
    pub fn set_mode(&mut self, mode: SamplerMode){
        self.sampler_mode = mode;
    }
    /// Sets the note for the warping to be based on
    pub fn set_warp_base(&mut self, base_note: u8){
        for voice in self.warp_voices.iter_mut(){
            voice.set_base_midi(base_note);
        }
    }
    /// Chooses a voice and steals the quietest one
    fn get_voice_id(&mut self)-> usize{
        for (voice_id, voice) in self.warp_voices.iter_mut().enumerate() {
            if !voice.is_active() {
                return voice_id;
            }
        }
        if let Some((quietest_voice_id, _)) = self
            .warp_voices
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
            .warp_voices
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

/// Fills a buffer with a file from a path
fn fill_warp_buffer(buffer: &mut RingBuffer<f32>, path: &str) ->f32{
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
    sample_rate as f32
}

fn create_buffer(path: &str)-> (RingBuffer<f32>,f32){
    // TODO: Add sample rate multiplier
    let mut reader = hound::WavReader::open(path).unwrap();
    let sample_format = reader.spec().sample_format;
    let num_channels = reader.spec().channels as usize;
    let sample_rate = reader.spec().sample_rate as f32;
    let length = reader.len();
    let mut buffer = RingBuffer::<f32>::new(length as usize);
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
    (buffer,sample_rate)
}