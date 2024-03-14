use crate::ring_buffer;
use ring_buffer::RingBuffer;
use hound::{WavReader, WavSpec, SampleFormat};
use std::io::BufReader;
use std::fs::File;
pub struct SampleVoice{
    spec: WavSpec,
    buffers: Vec<RingBuffer::<f32>>,
    phase_offsets: Vec<f32>,
    phase_step: f32,
    offset_midi: i8,
    bass_midi: u8,
    channel_id: usize,
}

impl SampleVoice{
    /// Takes in a WaveReader with an open file and the midi note that you want to place your sample at
    pub fn new(reader: &mut WavReader<BufReader<File>>, base_midi_: u8)->Self{
        // I want this line to split the vector by channels, so like this [left channel][right channel]
        // This way i can fill the buffers with their corresponding samples
        let spec = reader.spec();//(reader.len()/spec.channels as u32)as usize)
        let buffs_: Vec<RingBuffer<f32>> = (0..spec.channels as usize)
        .map(|_| RingBuffer::<f32>::new((reader.len()/spec.channels as u32)as usize))
        .collect();
        let offsets = vec![0.0;spec.channels as usize];
        
        let mut voice = SampleVoice{
            spec: spec,
            buffers: buffs_,
            phase_offsets: offsets,
            phase_step: 0.0,
            offset_midi: 0,
            bass_midi: base_midi_,
            channel_id: 0,
        };
        voice.fill_buffers(reader);

        voice
    }
    ///Reads from the internal buffer a the rate set by note_offset
    /// Uses the get_frac function in the ring_buffer, which returns the sample
    /// at a fractional index
    pub fn process(&mut self)->f32{
        self.channel_id =  self.channel_id % self.spec.channels as usize;
        let sample = self.buffers[self.channel_id].get_frac(self.phase_offsets[self.channel_id]);
        self.phase_offsets[self.channel_id] += self.phase_step;
        
        if self.phase_offsets[self.channel_id] >= self.buffers[0].capacity() as f32 {
            self.phase_step = 0.0;
            //self.phase_offsets[self.channel_id] -= self.buffers[0].capacity() as f32;
        }

        sample
    }

    ///Sets the midi note for the output
    /// 
    /// Is in reference to the base midi note
    pub fn set_note(&mut self, note: u8){
        self.offset_midi = note as i8 - self.bass_midi as i8;
        let offset = iclamp((self.offset_midi)as i32,-127,127);
        self.phase_step = 2.0_f32.powf(offset as f32 / 12.0) / self.spec.channels as f32;
    }
    /// Fills the internal buffers with the samples from the wav file
    /// 
    /// Takes into account the number of channels, bits per sample, and float vs int values.  
    fn fill_buffers(&mut self, reader: &mut WavReader<BufReader<File>>) {
        let sample_format = reader.spec().sample_format;
        let num_channels = self.spec.channels as usize;
        let sample_rate = self.spec.sample_rate as f32;
        let length = reader.len();

        // Determine the conversion factor based on sample format
        let conversion_factor = match sample_format {
            SampleFormat::Float => 1.0, // No conversion needed
            SampleFormat::Int => {
                match self.spec.bits_per_sample {
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
                for _ in 0..(length / num_channels as u32) {
                    for ch in 0..num_channels {
                        if let Some(sample) = samples.next() {
                            if let Ok(sample_value) = sample {
                                let sample_float = sample_value * conversion_factor;
                                self.buffers[ch].push(sample_float);
                            }
                        }
                    }
                }
            }, 
            SampleFormat::Int => {
                let mut samples = reader.samples::<i32>();
                for _ in 0..(length / num_channels as u32) {
                    for ch in 0..num_channels {
                        if let Some(sample) = samples.next() {
                            if let Ok(sample_value) = sample {
                                let sample_float = (sample_value as f32) * conversion_factor;
                                self.buffers[ch].push(sample_float);
                            }
                        }
                    }
                }
            }
        }

        // Calculates phase step based on sample rate
        self.phase_step =  2.0_f32.powf(self.offset_midi as f32 / 12.0) / num_channels as f32 ;
    }
}



fn fclamp(x: f32, min_val: f32, max_val: f32) -> f32 {
    if x < min_val {
        min_val
    } else if x > max_val {
        max_val
    } else {
        x
    }
}
fn iclamp(x: i32, min_val: i32, max_val: i32) -> i32 {
    if x < min_val {
        min_val
    } else if x > max_val {
        max_val
    } else {
        x
    }
}