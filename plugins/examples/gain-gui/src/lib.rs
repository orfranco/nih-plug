// nih-plug: plugins, but rewritten in Rust
// Copyright (C) 2022 Robbert van der Helm
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#[macro_use]
extern crate nih_plug;

use atomic_float::AtomicF32;
use nih_plug::{
    formatters, util, Buffer, BufferConfig, BusConfig, Editor, IntParam, Plugin, ProcessContext,
    ProcessStatus, Vst3Plugin,
};
use nih_plug::{FloatParam, Param, Params, Range, Smoother, SmoothingStyle};
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};
use std::pin::Pin;
use std::sync::Arc;

/// This is mostly identical to the gain example, minus some fluff, and with a GUI.
struct Gain {
    params: Pin<Arc<GainParams>>,
    editor_state: Arc<EguiState>,

    /// Needed to normalize the peak meter's response based on the sample rate.
    peak_meter_decay_weight: f32,
    /// The current data for the peak meter. This is stored as an [Arc] so we can share it between
    /// the GUI and the audio processing parts. If you have more state to share, then it's a good
    /// idea to put all of that in a struct behind a single `Arc`.
    ///
    /// This is stored as voltage gain.
    peak_meter: Arc<AtomicF32>,
}

#[derive(Params)]
struct GainParams {
    #[id = "gain"]
    pub gain: FloatParam,

    // TODO: Remove this parameter when we're done implementing the widgets
    #[id = "foobar"]
    pub some_int: IntParam,
}

impl Default for Gain {
    fn default() -> Self {
        Self {
            params: Arc::pin(GainParams::default()),
            editor_state: EguiState::from_size(300, 100),

            peak_meter_decay_weight: 1.0,
            peak_meter: Arc::new(AtomicF32::new(util::MINUS_INFINITY_DB)),
        }
    }
}

impl Default for GainParams {
    fn default() -> Self {
        Self {
            gain: FloatParam {
                value: 0.0,
                smoothed: Smoother::new(SmoothingStyle::Linear(50.0)),
                value_changed: None,
                range: Range::Linear {
                    min: -30.0,
                    max: 30.0,
                },
                name: "Gain",
                unit: " dB",
                value_to_string: formatters::f32_rounded(2),
                string_to_value: None,
            },
            some_int: IntParam {
                value: 3,
                smoothed: Smoother::none(),
                name: "Something",
                range: Range::Skewed {
                    min: 0,
                    max: 3,
                    factor: Range::skew_factor(1.0),
                },
                ..Default::default()
            },
        }
    }
}

impl Plugin for Gain {
    const NAME: &'static str = "Gain GUI";
    const VENDOR: &'static str = "Moist Plugins GmbH";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = "0.0.1";

    const DEFAULT_NUM_INPUTS: u32 = 2;
    const DEFAULT_NUM_OUTPUTS: u32 = 2;

    const ACCEPTS_MIDI: bool = false;

    fn params(&self) -> Pin<&dyn Params> {
        self.params.as_ref()
    }

    fn editor(&self) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let peak_meter = self.peak_meter.clone();
        create_egui_editor(
            self.editor_state.clone(),
            (),
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    // This is a fancy widget that can get all the information it needs to properly
                    // display and modify the parameter from the parametr itself
                    // It's not yet fully implemented, as the text is missing.
                    ui.label("Gain (now linked to some random skewed int)");
                    ui.add(widgets::ParamSlider::for_param(&params.some_int, setter));
                    ui.add(widgets::ParamSlider::for_param(&params.gain, setter));

                    // This is a simple naieve version of a parameter slider that's not aware of how
                    // the parmaeters work
                    ui.add(
                        egui::widgets::Slider::from_get_set(-30.0..=30.0, |new_value| {
                            match new_value {
                                Some(new_value) => {
                                    setter.begin_set_parameter(&params.gain);
                                    setter.set_parameter(&params.gain, new_value as f32);
                                    setter.end_set_parameter(&params.gain);
                                    new_value
                                }
                                None => params.gain.value as f64,
                            }
                        })
                        .suffix(" dB"),
                    );

                    // TODO: Add a proper custom widget instead of reusing a progress bar
                    let peak_meter =
                        util::gain_to_db(peak_meter.load(std::sync::atomic::Ordering::Relaxed));
                    let peak_meter_text = if peak_meter > util::MINUS_INFINITY_DB {
                        format!("{:.1} dBFS", peak_meter)
                    } else {
                        String::from("-inf dBFS")
                    };

                    let peak_meter_normalized = (peak_meter + 60.0) / 60.0;
                    ui.allocate_space(egui::Vec2::splat(2.0));
                    ui.add(
                        egui::widgets::ProgressBar::new(peak_meter_normalized)
                            .text(peak_meter_text),
                    );
                });
            },
        )
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // This works with any symmetrical IO layout
        config.num_input_channels == config.num_output_channels && config.num_input_channels > 0
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        buffer_config: &BufferConfig,
        _context: &mut impl ProcessContext,
    ) -> bool {
        // TODO: How do you tie this exponential decay to an actual time span?
        self.peak_meter_decay_weight = 0.9992f32.powf(44_100.0 / buffer_config.sample_rate);

        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _context: &mut impl ProcessContext,
    ) -> ProcessStatus {
        for samples in buffer.iter_mut() {
            let mut amplitude = 0.0;
            let num_samples = samples.len();

            let gain = self.params.gain.smoothed.next();
            for sample in samples {
                *sample *= util::db_to_gain(gain);
                amplitude += *sample;
            }

            // To save resources, a plugin can (and probably should!) only perform expensive
            // calculations that are only displayed on the GUI while the GUI is open
            if self.editor_state.is_open() {
                amplitude = (amplitude / num_samples as f32).abs();
                let current_peak_meter = self.peak_meter.load(std::sync::atomic::Ordering::Relaxed);
                let new_peak_meter = if amplitude > current_peak_meter {
                    amplitude
                } else {
                    current_peak_meter * self.peak_meter_decay_weight
                        + amplitude * (1.0 - self.peak_meter_decay_weight)
                };

                self.peak_meter
                    .store(new_peak_meter, std::sync::atomic::Ordering::Relaxed)
            }
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for Gain {
    const VST3_CLASS_ID: [u8; 16] = *b"GainGuiYeahBoyyy";
    const VST3_CATEGORIES: &'static str = "Fx|Dynamics";
}

nih_export_vst3!(Gain);