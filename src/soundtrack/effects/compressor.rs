use std::f32::consts::{SQRT_2, TAU};

use bevy_seedling::firewheel::{
    channel_config::ChannelConfig,
    core as firewheel_core,
    diff::{Diff, Patch},
    event::ProcEvents,
    node::{AudioNode, AudioNodeInfo, AudioNodeProcessor, ConstructProcessorContext, NodeError, ProcBuffers, ProcExtra, ProcInfo, ProcessStatus},
};

use crate::prelude::*;

#[derive(Reflect, Component, Clone, Copy, PartialEq, Diff, Patch)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct MultibandCompressor {
    pub freq_low_cutoff: f32,
    pub freq_high_cutoff: f32,
}

impl Default for MultibandCompressor {
    fn default() -> Self {
        Self {
            freq_low_cutoff: 100.,
            freq_high_cutoff: 2500.,
        }
    }
}

#[derive(Reflect, Component, Clone, Copy, PartialEq)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct MultibandCompressorConfig {
    pub channels: NonZeroChannelCount,
}

impl Default for MultibandCompressorConfig {
    fn default() -> Self {
        Self {
            channels: NonZeroChannelCount::STEREO,
        }
    }
}

impl AudioNode for MultibandCompressor {
    type Configuration = MultibandCompressorConfig;

    fn info(&self, configuration: &Self::Configuration) -> Result<AudioNodeInfo, NodeError> {
        Ok(AudioNodeInfo::new()
            .debug_name("multiband_compressor")
            .channel_config(ChannelConfig::new(configuration.channels.get(), configuration.channels.get())))
    }

    fn construct_processor(
        &self,
        _configuration: &Self::Configuration,
        _cx: ConstructProcessorContext,
    ) -> Result<impl AudioNodeProcessor, NodeError> {
        Ok(MultibandCompressorProcessor {
            params: self.clone(),
            processors: Vec::new(),
        })
    }
}

pub struct MultibandCompressorProcessor {
    params: MultibandCompressor,
    processors: Vec<ChannelProcessor>,
}

#[derive(Debug, Clone)]
struct ChannelProcessor {
    crossover: ThreeBandCrossover,
    low_comp: BandCompressor,
    mid_comp: BandCompressor,
    high_comp: BandCompressor,
}

impl AudioNodeProcessor for MultibandCompressorProcessor {
    fn events(&mut self, _info: &ProcInfo, events: &mut ProcEvents, _extra: &mut ProcExtra) {
        for patch in events.drain_patches::<MultibandCompressor>() {
            Patch::apply(&mut self.params, patch);
        }
    }

    fn process(&mut self, info: &ProcInfo, ProcBuffers { inputs, outputs }: ProcBuffers, _extra: &mut ProcExtra) -> ProcessStatus {
        if info.in_silence_mask.all_channels_silent(inputs.len()) {
            return ProcessStatus::ClearAllOutputs
        }

        for (i, (input, output)) in inputs.iter().zip(outputs.iter_mut()).enumerate() {
            self.processors.resize_with(self.processors.len().max(i + 1), || {
                let rate = info.sample_rate.get() as f32;
                ChannelProcessor {
                    crossover: ThreeBandCrossover::new(info.sample_rate, self.params.freq_low_cutoff, self.params.freq_high_cutoff),
                    low_comp: BandCompressor::new(
                        TransferFunction::new(vec![Vec2 { x: -80., y: -70. }, Vec2 { x: -30., y: -22. }, Vec2 { x: 0., y: -6. }]),
                        rate,
                        10.,
                        100.,
                    ),
                    mid_comp: BandCompressor::new(
                        TransferFunction::new(vec![Vec2 { x: -80., y: -65. }, Vec2 { x: -30., y: -20. }, Vec2 { x: 0., y: -4. }]),
                        rate,
                        5.,
                        50.,
                    ),
                    high_comp: BandCompressor::new(
                        TransferFunction::new(vec![Vec2 { x: -80., y: -60. }, Vec2 { x: -30., y: -18. }, Vec2 { x: 0., y: -3. }]),
                        rate,
                        2.,
                        25.,
                    ),
                }
            });

            let proc = &mut self.processors[i];
            for (input_sample, output_sample) in input.iter().zip(output.iter_mut()) {
                let [low, mid, high] = proc.crossover.process(*input_sample);
                *output_sample = proc.low_comp.process(low) + proc.mid_comp.process(mid) + proc.high_comp.process(high);
            }
        }

        ProcessStatus::OutputsModified
    }
}

#[derive(Debug, Clone, Copy)]
struct BiquadFilter {
    a: [f32; 2],
    b: [f32; 3],
    state: [f32; 2],
}

impl BiquadFilter {
    fn new(sample_rate: NonZeroU32, cutoff: f32, low_pass: bool) -> Self {
        let omega = TAU * cutoff / sample_rate.get() as f32;
        let alpha = omega.sin() / SQRT_2;
        let cos_omega = omega.cos();

        let a0 = 1. + alpha;
        Self {
            a: [(-2. * cos_omega) / a0, (1. - alpha) / a0],
            b: if low_pass {
                [(1. - cos_omega) / (2. * a0), (1. - cos_omega) / a0, (1. - cos_omega) / (2. * a0)]
            } else {
                [(1. + cos_omega) / (2. * a0), -(1. + cos_omega) / a0, (1. + cos_omega) / (2. * a0)]
            },
            state: [0., 0.],
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.b[0] * input + self.state[0];
        self.state[0] = self.b[1] * input - self.a[0] * output + self.state[1];
        self.state[1] = self.b[2] * input - self.a[1] * output;
        output
    }
}

#[derive(Debug, Clone, Copy)]
struct Lr4Filter {
    stages: [BiquadFilter; 2],
}

impl Lr4Filter {
    fn new(sample_rate: NonZeroU32, cutoff: f32, low_pass: bool) -> Self {
        Self {
            stages: [BiquadFilter::new(sample_rate, cutoff, low_pass); 2],
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let s = self.stages[0].process(input);
        self.stages[1].process(s)
    }
}

#[derive(Debug, Clone, Copy)]
struct ThreeBandCrossover {
    low: [Lr4Filter; 2],
    mid: [Lr4Filter; 2],
}

impl ThreeBandCrossover {
    fn new(sample_rate: NonZeroU32, freq_low_mid: f32, freq_mid_high: f32) -> Self {
        Self {
            low: [
                Lr4Filter::new(sample_rate, freq_low_mid, true),
                Lr4Filter::new(sample_rate, freq_low_mid, false),
            ],
            mid: [
                Lr4Filter::new(sample_rate, freq_mid_high, true),
                Lr4Filter::new(sample_rate, freq_mid_high, false),
            ],
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> [f32; 3] {
        let low = self.low[0].process(input);
        let mid_high_composite = self.low[1].process(input);

        [low, self.mid[0].process(mid_high_composite), self.mid[1].process(mid_high_composite)]
    }
}

#[derive(Debug, Clone)]
pub struct TransferFunction {
    points: Vec<Vec2>,
}

impl TransferFunction {
    pub fn new(points: Vec<Vec2>) -> Self {
        let mut this = Self { points };
        this.points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
        this
    }

    fn evaluate(&self, x_in: f32) -> f32 {
        if self.points.is_empty() {
            return x_in
        }

        if x_in <= self.points.first().unwrap().x {
            return self.points.first().unwrap().y
        }
        if x_in >= self.points.last().unwrap().x {
            return self.points.last().unwrap().y
        }

        let mut idx = 0;
        for i in 0..self.points.len() - 1 {
            if x_in >= self.points[i].x && x_in <= self.points[i + 1].x {
                idx = i;
                break
            }
        }

        let p0 = self.points[idx];
        let p1 = self.points[idx + 1];

        let t = (x_in - p0.x) / (p1.x - p0.x);
        p0.y + t * (p1.y - p0.y)
    }
}

#[derive(Debug, Clone)]
struct BandCompressor {
    transfer_func: TransferFunction,
    envelope: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl BandCompressor {
    fn new(transfer_func: TransferFunction, sample_rate: f32, attack_ms: f32, release_ms: f32) -> Self {
        Self {
            transfer_func,
            envelope: 0.,
            attack_coeff: (-1. / (attack_ms * 0.001 * sample_rate)).exp(),
            release_coeff: (-1. / (release_ms * 0.001 * sample_rate)).exp(),
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let input_abs = input.abs();
        if input_abs > self.envelope {
            self.envelope = input_abs + self.attack_coeff * (self.envelope - input_abs);
        } else {
            self.envelope = input_abs + self.release_coeff * (self.envelope - input_abs);
        }

        if self.envelope < 1e-6 {
            return input
        }

        let db_in = 20. * self.envelope.log10();
        let db_out = self.transfer_func.evaluate(db_in);
        let db_gain = db_out - db_in;
        let linear_gain = 10f32.powf(db_gain / 20.);

        input * linear_gain
    }
}

pub(super) fn plugin(app: &mut App) {
    app.register_node::<MultibandCompressor>();
}
