mod ring_buffer;
mod sample_voice;
use hound::WavSpec;
use sample_voice::SampleVoice;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut reader = hound::WavReader::open(&args[1]).unwrap();
    let wav_size = reader.len();
    let fill_buffer = vec![0.0;wav_size as usize];
    let base_note = 60;
    let mut sampVoice = SampleVoice::new(&mut reader, base_note);
    /* let mut voices: Vec<SampleVoice> = (0..5)
    .map(|_| SampleVoice::new(&mut reader, base_note))
    .collect(); */

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

    /* for (i,voice) in voices.iter_mut().enumerate(){
        voice.set_note(base_note + (i as u8 * 3));
    } */
    sampVoice.set_note(midi_note);
    
    let mut writer = hound::WavWriter::create(&args[2], newSpec).unwrap();
    for i in (0..wav_size){
        /* let mut outSample = 0.0;
        for voice in voices.iter_mut(){
            outSample += voice.process();
        } */
        let outSample = sampVoice.process();
        writer.write_sample(outSample);
    }
    writer.finalize().unwrap();
    println!("This is your sample {} semitones apart", midi_note as i8 -base_note as i8);
    //println!("This is your sample... but with chords now!!");
}
