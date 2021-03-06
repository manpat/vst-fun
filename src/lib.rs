use vst::plugin::{Info, CanDo, Plugin, Category as VstCategory};
use vst::editor::Editor;
use vst::buffer::AudioBuffer as VstAudioBuffer;
use vst::api::Supported as VstSupported;
use vst::api::Events as VstEvents;

mod model;
mod view;

use model::Model;
use view::View;

struct BasicPlugin {
    synth_view: View,
    synth_ctx: voi_synth::Context,

    prev_model: Model,

    parameters: Vec<voi_synth::ParameterID>,
    num_keys_down: u32,
}

impl Plugin for BasicPlugin {
    fn get_info(&self) -> Info {
        log::info!("get_info");

        let parameters = self.prev_model.parameter_map().len() as _;

        Info {
            name: "GUI Test".to_string(),
            vendor: "_manpat".to_string(),
            unique_id: 20190303,

            category: VstCategory::Synth,

            parameters,
            outputs: 1,

            ..Info::default()
        }
    }

    fn get_editor(&mut self) -> Option<&mut dyn Editor> { Some(&mut self.synth_view) }

    fn can_do(&self, can_do: CanDo) -> VstSupported {
        match can_do {
            CanDo::ReceiveMidiEvent => VstSupported::Yes,
            _ => VstSupported::Maybe,
        }
    }

    fn set_block_size(&mut self, size: i64) { self.synth_ctx.set_buffer_size(size as _) }
    fn set_sample_rate(&mut self, rate: f32) { self.synth_ctx.set_sample_rate(rate) }

    fn get_parameter(&self, index: i32) -> f32 {
        self.prev_model.parameter_map()
            .get(index as usize)
            .map(|&(_, v)| v)
            .unwrap_or(0.0)
    }

    fn set_parameter(&mut self, index: i32, val: f32) {
        if let Some(pv) = self.prev_model.parameter_map_mut().get_mut(index as usize) {
            *pv.1 = val;
        }
    }

    fn process(&mut self, out_buf: &mut VstAudioBuffer<f32>) {
        assert!(out_buf.output_count() == 1);

        // NOTE: this is probably too late
        let model = self.synth_view.model();

        for (pos, val) in model.diff_with(self.prev_model) {
            self.synth_ctx.set_parameter(self.parameters[pos], val);
        }

        self.prev_model = model;



        let buf = self.synth_ctx.get_ready_buffer().expect("Failed to get ready buffer");

        if buf.len() == out_buf.samples() {
            let out_buf = out_buf.split().1.get_mut(0);
            buf.copy_to(out_buf);
        } else {
            log::warn!("Buffer size mismatch in plugin process");
        }

        self.synth_ctx.queue_empty_buffer(buf).unwrap();
    }

    fn process_events(&mut self, events: &VstEvents) {
        use vst::event::Event;

        for event in events.events() {
            match event {
                Event::Midi(midi_event) => self.process_midi_event(midi_event),
                _ => {}
            }
        }
    }
}


impl Default for BasicPlugin {
    fn default() -> Self {
        use voi_synth::NodeContainer;

        let synth_ctx = voi_synth::Context::new(3, 256).unwrap();

        let mut s = voi_synth::Synth::new();
        s.set_gain(0.3);

        let freq = s.new_parameter();
        let trigger = s.new_parameter();

        let cutoff_dip = s.new_parameter();
        let lfo_depth = s.new_parameter();
        let filter_lfo_depth = s.new_parameter();
        let wonk_a = s.new_parameter();
        let wonk_b = s.new_parameter();

        let feedback = s.new_value_store();

        let feedback_a = s.new_multiply(wonk_a, feedback);
        let feedback_b = s.new_multiply(wonk_b, feedback);
        let feedback_b = s.new_multiply(feedback_b, freq);

        let env = s.new_env_adsr(0.01,  0.1, 0.8,  0.8, trigger);
        let env = s.new_power(env, 2.0);

        let lfo = s.new_sine(6.0);
        let lfo = s.new_multiply(lfo, lfo_depth);
        let osc_freq = s.new_add(lfo, freq);
        let osc_freq = s.new_add(osc_freq, feedback_b);

        let osc1 = s.new_triangle(osc_freq);
        let osc2 = s.new_square(osc_freq);
        let osc3 = s.new_saw(osc_freq);

        let osc_mix_a = s.new_sub(0.0, feedback_a);
        let osc_mix_a = s.new_clamp(osc_mix_a, 0.0, 1.0);
        let osc_mix_b = s.new_clamp(feedback_a, 0.0, 1.0);
        let osc = s.new_mix(osc1, osc2, osc_mix_a);
        let osc = s.new_mix(osc, osc3, osc_mix_b);

        let filter_env = s.new_env_adsr(0.2,  0.0, 1.0,  0.5, trigger);
        let filter_env = s.new_power(filter_env, 8.0);

        let filter_lfo = s.new_sine(6.0);
        let filter_lfo = s.new_multiply(filter_lfo, filter_lfo_depth);

        let filter_freq = s.new_multiply(filter_env, cutoff_dip);
        let filter_freq = s.new_sub(1000.0, filter_freq);
        let filter_freq = s.new_add(filter_freq, filter_lfo);
        let filter_freq = s.new_add(filter_freq, feedback_b);

        let filter = s.new_lowpass(osc, filter_freq);
        let filter = s.new_lowpass(filter, filter_freq);

        let bass_freq = s.new_multiply(osc_freq, 0.251);
        let osc_bass = s.new_triangle(bass_freq);

        let out = s.new_add(filter, osc_bass);
        s.new_store_write(feedback, out);

        let out = s.new_multiply(env, out);

        s.set_output(out);
        s.get_parameter(freq).set_value(440.0);

        synth_ctx.push_synth(s).unwrap();

        let parameters = vec![
            freq, trigger, cutoff_dip,
            lfo_depth, filter_lfo_depth,
            wonk_a, wonk_b,
        ];

        let model = Model::default();
        for (pos, val) in model.parameter_iter() {
            synth_ctx.set_parameter(parameters[pos], val);
        }

        BasicPlugin {
            synth_view: View::new(model),
            synth_ctx,
            prev_model: model,

            parameters,

            num_keys_down: 0,
        }
    }
}


impl BasicPlugin {
    fn process_midi_event(&mut self, evt: vst::event::MidiEvent) {
        let packet = evt.data;

        match packet[0] {
            0x80 ..= 0x8F => self.note_off(packet[1]),
            0x90 ..= 0x9F => {
                let key = packet[1];
                let velocity = packet[2];

                if velocity > 0 {
                    self.note_on(key, velocity);
                } else {
                    self.note_off(key);
                }
            }

            _ => {}
        }
    }

    fn note_on(&mut self, key: u8, velocity: u8) {
        let freq = 440.0 * 2.0f32.powf((key as f32 - 64.0) / 12.0);
        self.synth_ctx.set_parameter(self.parameters[0], freq);
        self.synth_ctx.set_parameter(self.parameters[1], velocity as f32 / 127.0);

        self.num_keys_down += 1;
    }

    fn note_off(&mut self, key: u8) {
        self.num_keys_down = self.num_keys_down.saturating_sub(1);
        if self.num_keys_down == 0 {
            self.synth_ctx.set_parameter(self.parameters[1], 0.0);
        }
    }
}


vst::plugin_main!(BasicPlugin);
