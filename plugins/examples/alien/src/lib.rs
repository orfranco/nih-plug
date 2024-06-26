use crate::sensors_data_receiver::SensorData;
use nih_plug::prelude::NoteEvent;
use nih_plug::prelude::*;
use nih_plug::{nih_error, nih_log};
use nih_plug_iced::IcedState;
use sensors_data_receiver::SensorDataReceiver;
use std::sync::Arc;

mod editor;
mod sensors_data_receiver;

#[derive(Params)]
struct AlienParams {
    #[persist = "editor-state"]
    editor_state: Arc<IcedState>,

    #[id = "cc_value"]
    cc_value: FloatParam,
}

struct Alien {
    params: Arc<AlienParams>,
    sensors_data_receiver: SensorDataReceiver,
}

impl Default for AlienParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            cc_value: FloatParam::new(
                "CC Value",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(1.0),
                },
            ),
        }
    }
}

impl Default for Alien {
    fn default() -> Self {
        Self {
            params: Arc::new(AlienParams::default()),
            sensors_data_receiver: SensorDataReceiver::new(),
        }
    }
}

impl Plugin for Alien {
    const NAME: &'static str = "Alien";
    const VENDOR: &'static str = "Alien";
    const URL: &'static str = "https://mycompany.com";
    const EMAIL: &'static str = "support@mycompany.com";

    const VERSION: &'static str = "0.1.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(1),
        main_output_channels: NonZeroU32::new(1),
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("Initializing Alien...");

        match self.sensors_data_receiver.initialize() {
            Ok(_) => {
                nih_log!("Successfully initialized Sensors Data Receiver.");
                true
            }
            Err(err) => {
                nih_error!("Failed initializing Sensors Data Receiver. Error: {err}");
                false
            }
        }
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let lock_res = self.sensors_data_receiver.curr_data.lock().map_err(|err| {
            nih_error!("Failed locking curr_data while processing message. Error: {err}")
        });

        if let Ok(unlocked_data) = lock_res {
            for (_key, data) in unlocked_data.iter() {
                self.calculate_channel_values_and_send(context, data).map_err(|err| nih_error!("Failed Calculating Channel Values and sending midi Messages Containing them. Error: {err}")).ok();
            }
        }

        ProcessStatus::Normal
    }
}

impl Alien {
    fn calculate_channel_values_and_send(
        &self,
        context: &mut impl ProcessContext<Self>,
        sensor_data: &SensorData,
    ) -> Result<(), u8> {
        // TODO: implement a better way to decide on which channel to send each sensor_id data.
        // 1_a sent to 7-8-9, 1_b sent to 10-11-12:
        let mut channel: u8 = 7;
        if sensor_data.sensor_id == "1_b" {
            channel = 10;
        }

        self.send_midi_message(
            context,
            channel,
            (sensor_data.euler_x * 127.0 / 360.0) as u8,
        )?;
        self.send_midi_message(
            context,
            channel + 1,
            (sensor_data.euler_y * 127.0 / 360.0) as u8,
        )?;
        self.send_midi_message(
            context,
            channel + 2,
            (sensor_data.euler_z * 127.0 / 360.0) as u8,
        )?;

        Ok(())
    }

    fn send_midi_message(
        &self,
        context: &mut impl ProcessContext<Self>,
        channel: u8,
        value: u8,
    ) -> Result<(), u8> {
        let midi_data = [
            0xB0,    // Control Change message on channel 1
            channel, // CC number
            value,   // CC value (0-127)
        ];

        let midi_message = NoteEvent::from_midi(0, &midi_data)?;
        context.send_event(midi_message);

        Ok(())
    }
}

impl ClapPlugin for Alien {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.gain-gui-iced";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A smoothed gain parameter example plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Alien {
    const VST3_CLASS_ID: [u8; 16] = *b"M1d1Inv3r70rzAaA";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx];
}

nih_export_clap!(Alien);
nih_export_vst3!(Alien);
