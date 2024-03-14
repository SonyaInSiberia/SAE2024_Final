use crate::ring_buffer;
use ring_buffer::RingBuffer;

pub struct SampleVoice{
    phase_offset: f32,
    phase_step: f32,
    offset_midi: i8,
    bass_midi: u8,
    num_channels: usize,
}

impl SampleVoice{
    /// Takes in a WaveReader with an open file and the midi note that you want to place your sample at
    pub fn new(num_channesls_: usize, base_midi_: u8)->Self{
        // I want this line to split the vector by channels, so like this [left channel][right channel]
        // This way i can fill the buffers with their corresponding samples
        
        let mut voice = SampleVoice{
            phase_offset: 0.0,
            phase_step: 1.0,
            offset_midi: 0,
            bass_midi: base_midi_,
            num_channels: num_channesls_,
        };
        //voice.fill_buffers(reader);

        voice
    }
    ///Reads from the internal buffer a the rate set by note_offset
    /// Uses the get_frac function in the ring_buffer, which returns the sample
    /// at a fractional index
    pub fn process(&mut self, buffer: &mut RingBuffer<f32>)->f32{
        let sample = buffer.get_frac(self.phase_offset);
        self.phase_offset += self.phase_step;
        
        if self.phase_offset >= buffer.capacity() as f32 {
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
        self.phase_step = 2.0_f32.powf(offset as f32 / 12.0);
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