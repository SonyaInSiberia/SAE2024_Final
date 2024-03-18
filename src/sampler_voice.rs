use std::clone;

use crate::ring_buffer;
use ring_buffer::RingBuffer;
use crate::adsr;
use adsr::ADSR;
#[derive(Clone)]
pub struct SamplerVoice{
    phase_offset: f32,
    phase_step: f32,
    pub midi_note: u8,
    bass_midi: u8,
    num_channels: usize,
    pub adsr: ADSR,
    pub sus_is_velo: bool,
}

impl SamplerVoice{
    /// Takes in a WaveReader with an open file and the midi note that you want to place your sample at
    pub fn new(num_channesls_: usize, base_midi_: u8)->Self{
        let adsr_ = ADSR::new(44100.0, 0.2, 0.1,0.5,0.2);
        let mut voice = SamplerVoice{
            phase_offset: 0.0,
            phase_step: 1.0,
            midi_note: 0,
            bass_midi: base_midi_,
            num_channels: num_channesls_,
            adsr: adsr_,
            sus_is_velo: false,

        };
        voice
    }
    ///Reads from the loaded sample file
    /// Uses the get_frac function in the ring_buffer, which returns the sample
    /// at a fractional index
    pub fn processWarp(&mut self, buffer: &mut RingBuffer<f32>)->f32{
        if self.adsr.is_active(){
            let sample = buffer.get_frac(self.phase_offset);
            self.phase_offset += self.phase_step;
               
            if self.phase_offset >= buffer.capacity() as f32 {
                self.phase_step = 0.0;
                self.phase_offset = 0.0;
                //self.phase_offsets[self.channel_id] -= self.buffers[0].capacity() as f32;
            }
            sample * self.adsr.getNextSample()
        }else{
            self.phase_offset = 0.0;
            self.phase_step = 0.0;
            0.0
        }
    }

    pub fn processAssign(&mut self, buffer: &mut RingBuffer<f32>)->f32{
        if self.adsr.is_active(){
            buffer.pop()*self.adsr.getNextSample()
        }else{
            buffer.set_read_index(0);
            0.0
        }
    }
    ///Sets the midi note for the output
    /// 
    /// Is in reference to the base midi note
    pub fn set_note(&mut self, note: u8){
        self.midi_note = note;
        let offset = iclamp((note as i8 - self.bass_midi as i8)as i32,-127,127);
        self.phase_step = 2.0_f32.powf(offset as f32 / 12.0);
    }

    pub fn note_on(&mut self, note: u8, velocity: u8){
        if self.sus_is_velo {
            let float_velo = velocity as f32 / 127.0;
            self.adsr.set_sustain(float_velo);
        }
        self.set_note(note);
        self.adsr.note_on();
    }

    pub fn note_off(&mut self){
        self.adsr.note_off()
    }

    pub fn set_adsr(&mut self, attack_:f32, decay_:f32, sustain_:f32, release_:f32){
        if !self.sus_is_velo{
            self.adsr.set_sustain(sustain_);
        }
        self.adsr.set_attack(attack_);
        self.adsr.set_decay(decay_);
        self.adsr.set_release(release_);
    }

    pub fn is_active(&mut self)->bool{
        self.adsr.is_active()
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