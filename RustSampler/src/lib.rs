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
            editor_state: EguiState::from_size(300, 180),
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
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
                    // NOTE: See `plugins/diopser/src/editor.rs` for an example using the generic UI widget

                    // This is a fancy widget that can get all the information it needs to properly
                    // display and modify the parameter from the parametr itself
                    // It's not yet fully implemented, as the text is missing.
                    ui.label("Some random integer");
                    egui_ctx
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
        self.engine.as_mut().unwrap().load_file_from_path("/Users/davidjones/Desktop/0My_samples/808_drum_kit/808_drum_kit/classic 808/1 weird 808.wav");
        self.engine.as_mut().unwrap().add_to_paths_and_load("/Users/davidjones/Desktop/0My_samples/Cymatics - Fantasy Synth Sample Pack/One Shots/Cymatics - Fantasy - Juno 106 BASS Rubber - C.wav");
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
