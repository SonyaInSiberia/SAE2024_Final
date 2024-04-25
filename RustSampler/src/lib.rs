use nih_plug::prelude::*;
use std::sync::Arc;
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};
mod adsr;
mod ring_buffer;
mod sampler_voice;
mod sampler_engine;
mod crossfade;
use sampler_engine::{SamplerEngine,SamplerMode};
use sampler_voice::SustainModes;
use egui::{ColorImage, ImageData, TextureHandle, TextureOptions, Context as EguiContext, Color32};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};

use std::fs;




// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct RustSampler {
    params: Arc<RustSamplerParams>,
    engine: Option<SamplerEngine>,
}

#[derive(Params)]
struct RustSamplerParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
    #[id = "release"]
    pub release: FloatParam,
    #[id = "start_point"]
    pub start_point: FloatParam,
    #[id = "end_point"]
    pub end_point: FloatParam,
    #[id = "num_voices"]
    pub num_voices: IntParam,
    #[id = "sus_start"]
    pub sus_start: FloatParam,
    #[id = "sus_end"]
    pub sus_end: FloatParam,
    #[id = "sus_mode"]
    pub sus_mode: EnumParam<SustainModes>,
    #[id = "fade_time"]
    pub fade_time: FloatParam,
}

impl Default for RustSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(RustSamplerParams::default()),
            engine: None,

        }
    }
}



impl Default for RustSamplerParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(800, 600),
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.

            
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-70.0),
                    max: util::db_to_gain(6.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 6.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            attack: FloatParam::new(
                "Attack",
                0.0, 
                FloatRange::Linear { min: 0.0, max: 1000.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("ms"),
            decay: FloatParam::new(
                "Decay",
                100.0, 
                FloatRange::Linear { min: 0.0, max: 1000.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("ms"),
            sustain: FloatParam::new(
                "Sustain",
                1.0, 
                FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(20.0)),
            release: FloatParam::new(
                "Release",
                200.0, 
                FloatRange::Linear { min: 0.0, max: 2000.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("ms"),
            start_point: FloatParam::new(
                "Start Point",
                0.0, 
                FloatRange::Linear { min: 0.0, max: 100.0})
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("%")
                .with_step_size(0.001),
            end_point: FloatParam::new(
                "End Point",
                100.0, 
                FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("%")
                .with_step_size(0.001),
            num_voices: IntParam::new( //Max Number of Voices
                "Voices",
                6,
                IntRange::Linear { min: 1, max: 24 }
            ),
            sus_start: FloatParam::new(
                "Sustain Start",
                40.0, 
                FloatRange::Linear { min: 0.0, max: 100.0})
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("%")
                .with_step_size(0.001),
            sus_end: FloatParam::new(
                "Sustain End",
                60.0, 
                FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("%")
                .with_step_size(0.001),
            sus_mode: EnumParam::new(
                "Sustain Mode",
                SustainModes::NoLoop,
            ),
            fade_time: FloatParam::new(
                "Crossfade time",
                0.0, 
                FloatRange::Linear { min: 0.0, max: 500.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit("ms")
                .with_step_size(1.0),
        }
    }
}

impl Plugin for RustSampler {
    const NAME: &'static str = "RustSampler";
    const VENDOR: &'static str = "ASE Group 2";
    const URL: &'static str =  env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "davidisjones10.gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];


    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }


    
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    // Handle the gain slider
                    let mut gain_db = util::gain_to_db(params.gain.value());
                    let slider = egui::Slider::new(&mut gain_db, -70.0..=6.0).text("Gain");
                    let response = ui.add_sized([200.0, 40.0], slider);
    
                    if response.changed() {
                        setter.set_parameter(&params.gain, util::db_to_gain(gain_db));
                    }
                    
                    // Handle the attack slider
                    let mut attack = params.attack.value();
                    let attack_slider = egui::Slider::new(&mut attack, 0.0..=1000.0).text("Attack (ms)");
                    if ui.add(attack_slider).changed() {
                        setter.set_parameter(&params.attack, attack);
                    }

                    // Handle the decay slider
                    let mut decay = params.decay.value();
                    let decay_slider = egui::Slider::new(&mut decay, 0.0..=1000.0).text("Decay (ms)");
                    if ui.add(decay_slider).changed() {
                        setter.set_parameter(&params.decay, decay);
                    }

                    // Handle the sustain slider
                    let mut sustain = params.sustain.value();
                    let sustain_slider = egui::Slider::new(&mut sustain, 0.0..=1.0).text("Sustain");
                    if ui.add(sustain_slider).changed() {
                        setter.set_parameter(&params.sustain, sustain);
                    }

                    // Handle the release slider
                    let mut release = params.release.value();
                    let release_slider = egui::Slider::new(&mut release, 0.0..=2000.0).text("Release (ms)");
                    if ui.add(release_slider).changed() {
                        setter.set_parameter(&params.release, release);
                    }

                    // Additional parameters...
                    // Example for start_point and end_point
                    let mut start_point = params.start_point.value();
                    let start_point_slider = egui::Slider::new(&mut start_point, 0.0..=100.0).text("Start Point (%)");
                    if ui.add(start_point_slider).changed() {
                        setter.set_parameter(&params.start_point, start_point);
                    }

                    let mut end_point = params.end_point.value();
                    let end_point_slider = egui::Slider::new(&mut end_point, 0.0..=100.0).text("End Point (%)");
                    if ui.add(end_point_slider).changed() {
                        setter.set_parameter(&params.end_point, end_point);
                    }
    
                    // Handle the image
                    let image_path = "/Users/jiaheqian/Desktop/funny-background-drawing-backgrounds-cartoon-1-5c9b97c68e299.png";
                    let image_data = std::fs::read(image_path).expect("Failed to read image file");
                    let image = image::load_from_memory(&image_data).expect("Failed to load image");
    
                    let (width, height) = image.dimensions();
                    let rgba_image = image.to_rgba8();
    
                    let color_pixels = rgba_image
                        .pixels()
                        .map(|p| egui::Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3]))
                        .collect::<Vec<_>>();
    
                    let color_image = egui::ColorImage {
                        size: [width as usize, height as usize],
                        pixels: color_pixels,
                    };
    
                    let image_data = ImageData::from(color_image);
                    let options = TextureOptions::default();
                    let texture = egui_ctx.load_texture("background_image", image_data, options);
    
                    // Show the image
                    ui.image((texture.id(), texture.size_vec2())); // Correct usage: as a tuple
                });
            },
        )
    }
    
    
    


    
    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        let engine_ = SamplerEngine::new(_buffer_config.sample_rate, 2);
        self.engine = Some(engine_);
        // Tests to see if second file will overwrite first file
        self.engine.as_mut().unwrap().load_file_from_path("/Users/jiaheqian/Desktop/Rust Sample/Interaction_PoseDetection_Scissors_02.wav");
        self.engine.as_mut().unwrap().add_to_paths_and_load("/Users/jiaheqian/Desktop/Rust Sample/Interaction_PoseDetection_Stop_01.wav");
        self.engine.as_mut().unwrap().set_mode(SamplerMode::Warp);
        self.engine.as_mut().unwrap().set_warp_base(64);
        self.engine.as_mut().unwrap().set_warp_base(60);
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            // TODO: Find out why no audio... not getting midi messages
            while let Some(event) = next_event{
                match event{
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        self.engine.as_mut().unwrap().note_on(note, velocity);
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        self.engine.as_mut().unwrap().note_off(note);
                    }
                    _ => (),
                }
                next_event = context.next_event();
            }
            for sample in channel_samples {
                let gain = self.params.gain.smoothed.next();
                let attack = self.params.attack.smoothed.next()*0.001;
                let decay = self.params.decay.smoothed.next()*0.001;
                let sustain = self.params.sustain.smoothed.next();
                let release = self.params.release.smoothed.next()*0.001;
                let num_voices = self.params.num_voices.value();
                let start = self.params.start_point.smoothed.next();
                let end = self.params.end_point.smoothed.next();
                let sus_start = self.params.sus_start.smoothed.next();
                let sus_end = self.params.sus_end.smoothed.next();
                let sus_mode = self.params.sus_mode.value();
                let fade_time = self.params.fade_time.value()*0.001;
                self.engine.as_mut().unwrap().set_num_voices(num_voices as u8);
                self.engine.as_mut().unwrap().set_adsr_warp(attack, decay, sustain, release);
                self.engine.as_mut().unwrap().set_points_warp(start, end);
                self.engine.as_mut().unwrap().set_sus_looping_warp(sus_mode);
                self.engine.as_mut().unwrap().set_sus_points_warp(sus_start, sus_end);
                self.engine.as_mut().unwrap().set_fade_time_warp(fade_time);
                *sample = self.engine.as_mut().unwrap().process();
                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for RustSampler {
    const CLAP_ID: &'static str = "RustSampler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A sampler in Rust");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo, ClapFeature::Instrument];
}

impl Vst3Plugin for RustSampler {
    const VST3_CLASS_ID: [u8; 16] = *b"Rust_Sampler_VST";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics, 
        Vst3SubCategory::Generator,Vst3SubCategory::Instrument];
}

nih_export_clap!(RustSampler);
nih_export_vst3!(RustSampler);
