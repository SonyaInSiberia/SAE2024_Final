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
use nih_plug::prelude::*;
use egui::epaint::{PathShape, Pos2, Stroke, Rect};
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

                egui::Window::new("ADSR Curve")
                // .vscroll(false)
                // .resizable(false)
                .default_size(egui::Vec2::new(200.0, 100.0))
                .show(egui_ctx, |ui| {
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        let (response, painter) =
                            ui.allocate_painter(egui::Vec2::new(ui.available_width(), 200.0), egui::Sense::hover());

                        let attack = params.attack.value();
                        let decay = params.decay.value();
                        let sustain = params.sustain.value();
                        let release = params.release.value();

                        let total_duration = 5000.0;

                        let to_screen = egui::emath::RectTransform::from_to(
                            egui::Rect::from_min_max(
                                Pos2::new(0.0, 0.0),
                                Pos2::new(total_duration, 1.0),
                            ),
                            response.rect,
                        );

                        let mut points = Vec::new();
                        let num_points = 100;

                        // Attack phase
                        for i in 0..num_points {
                            let t = i as f32 / num_points as f32 * attack;
                            let y = 1.0 - (t / attack);
                            points.push(Pos2::new(t, y));
                        }

                        // Decay phase
                        let start_time = attack;
                        let end_time = start_time + decay;
                        for i in 0..num_points {
                            let t = start_time + i as f32 / num_points as f32 * (end_time - start_time);
                            let y = (1.0 - sustain) * i as f32 / num_points as f32;
                            points.push(Pos2::new(t, y));
                        }

                        // Sustain phase
                        let start_time = attack + decay;
                        let end_time = total_duration - release;
                        for i in 0..num_points {
                            let t = start_time + i as f32 / num_points as f32 * (end_time - start_time);
                            let y = 1.0 - sustain;
                            points.push(Pos2::new(t, y));
                        }

                        // Release phase
                        let start_time = total_duration - release;
                        for i in 0..num_points {
                            let t = start_time + i as f32 / num_points as f32 * release;
                            let y = (1.0 - sustain) + i as f32 / num_points as f32 * sustain;
                            points.push(Pos2::new(t, y));
                        }

                        let stroke = Stroke::new(1.0, Color32::from_rgb(50, 100, 150));
                        let points_in_screen: Vec<Pos2> = points.iter().map(|p| to_screen * *p).collect();
                        let path = PathShape::line(points_in_screen, stroke);
                        painter.add(path);
                    });
                });



                egui::CentralPanel::default().show(egui_ctx, |ui| {

                    /// ADSR
                    ui.label("Attack");
                    ui.add(widgets::ParamSlider::for_param(&params.attack, setter));
                    ui.label("Decay");
                    ui.add(widgets::ParamSlider::for_param(&params.decay, setter));
                    ui.label("Sustain");
                    ui.add(widgets::ParamSlider::for_param(&params.sustain, setter));
                    ui.label("Release");
                    ui.add(widgets::ParamSlider::for_param(&params.release, setter));



                    // Handle the gain slider
                    let mut gain_db = util::gain_to_db(params.gain.value());
                    let slider = egui::Slider::new(&mut gain_db, -70.0..=6.0).text("Gain");
                    let response = ui.add_sized([200.0, 40.0], slider);
    
                    if response.changed() {
                        setter.set_parameter(&params.gain, util::db_to_gain(gain_db));
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


                    // Handle the num_voices slider
                    let mut num_voices = params.num_voices.value() as i32;  // Casting to i32 for the slider
                    let num_voices_slider = egui::Slider::new(&mut num_voices, 1..=24).text("Number of Voices");
                    if ui.add(num_voices_slider).changed() {
                        setter.set_parameter(&params.num_voices, num_voices as i32);  // Cast back to i32 if needed
                    }

                    // Handle the sus_start slider
                    let mut sus_start = params.sus_start.value();
                    let sus_start_slider = egui::Slider::new(&mut sus_start, 0.0..=100.0).text("Sustain Start (%)");
                    if ui.add(sus_start_slider).changed() {
                        setter.set_parameter(&params.sus_start, sus_start);
                    }

                    // Handle the sus_end slider
                    let mut sus_end = params.sus_end.value();
                    let sus_end_slider = egui::Slider::new(&mut sus_end, 0.0..=100.0).text("Sustain End (%)");
                    if ui.add(sus_end_slider).changed() {
                        setter.set_parameter(&params.sus_end, sus_end);
                    }


                    ui.label("Sustain Mode");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut params.sus_mode.value(), SustainModes::NoLoop, "No Loop");
                        ui.selectable_value(&mut params.sus_mode.value(), SustainModes::LoopWrap, "Loop Wrap");
                        ui.selectable_value(&mut params.sus_mode.value(), SustainModes::LoopBounce, "Loop Bounce");
                    });
                    ui.end_row();
                    // Handle the fade_time slider
                    let mut fade_time = params.fade_time.value();
                    let fade_time_slider = egui::Slider::new(&mut fade_time, 0.0..=500.0).text("Crossfade Time (ms)");
                    if ui.add(fade_time_slider).changed() {
                        setter.set_parameter(&params.fade_time, fade_time);
                    }



    
                    // Handle the image
                    let image_path = "/Users/jiaheqian/Downloads/DALL·E 2024-04-25 02.40.14 - A detailed retro-style illustration of a music sampler with numerous knobs and buttons, depicting a complex old-school mixing environment. Include vin.webp";
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
        self.engine.as_mut().unwrap().load_file_from_path("/Users/carsonzhang/Desktop/GT/SP24/MUSI 6106 Audio Software Engineering/SAE2024_Final/RustSampler/audio/guitar.wav");
        self.engine.as_mut().unwrap().add_to_paths_and_load("/Users/carsonzhang/Desktop/GT/SP24/MUSI 6106 Audio Software Engineering/SAE2024_Final/RustSampler/audio/sweep.wav");
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