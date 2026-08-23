use crate::prelude::*;

#[derive(Reflect, Component, Clone, PartialEq, Diff, Patch)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct MultibandCompressor {
    pub freq_low_cut: f32,
    pub freq_low_cutoff: f32,
    pub freq_high_cutoff: f32,
    pub low: BandCompressor,
    pub mid: BandCompressor,
    pub high: BandCompressor,
    pub master: BandCompressor,
    pub low_stereo_separation: f32,
    pub mid_stereo_separation: f32,
    pub high_stereo_separation: f32,
    pub master_stereo_separation: f32,
}

impl Default for MultibandCompressor {
    fn default() -> Self {
        // Maximus, Soundgoodizer Preset A
        Self {
            freq_low_cut: 20.,
            freq_low_cutoff: 200.,
            freq_high_cutoff: 3000.,
            low: BandCompressor {
                curve: BandCurve::new(vec![vec3(-80., -80., 0.), vec3(0., 0., 0.07), vec3(12., 0., 0.)]),
                attack_ms: 2.,
                release_ms: 137.48,
                sustain_ms: 10.,
                pre_gain: Volume::Decibels(5.),
                post_gain: Volume::Decibels(5.6),
            },
            mid: BandCompressor {
                curve: BandCurve::new(vec![vec3(-80., -80., 0.), vec3(-3., -3., 0.), vec3(12., 2.7, 0.19)]),
                attack_ms: 2.,
                release_ms: 85.53,
                sustain_ms: 3.31,
                pre_gain: Volume::Decibels(6.4),
                post_gain: Volume::Decibels(0.),
            },
            high: BandCompressor {
                curve: BandCurve::new(vec![vec3(-80., -80., 0.), vec3(0., 0., 0.14), vec3(12., 0., 0.)]),
                attack_ms: 2.,
                release_ms: 85.53,
                sustain_ms: 2.18,
                pre_gain: Volume::Decibels(6.9),
                post_gain: Volume::Decibels(2.9),
            },
            master: BandCompressor {
                curve: BandCurve::new(vec![vec3(-80., -80., 0.), vec3(0., 0., 0.), vec3(12., 2.7, 0.085)]),
                attack_ms: 2.,
                release_ms: 85.53,
                sustain_ms: 3.2,
                pre_gain: Volume::Decibels(0.),
                post_gain: Volume::Decibels(0.),
            },
            low_stereo_separation: -1.,
            mid_stereo_separation: 0.38,
            high_stereo_separation: 0.,
            master_stereo_separation: 0.,
        }
    }
}

#[derive(Reflect, Debug, Clone, PartialEq, Diff, Patch)]
#[reflect(Debug, Clone, PartialEq)]
pub struct BandCompressor {
    pub curve: BandCurve,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub sustain_ms: f32,
    pub pre_gain: Volume,
    pub post_gain: Volume,
}

#[derive(Reflect, Debug, Clone, PartialEq, Diff, Patch)]
#[reflect(Debug, Clone, PartialEq)]
pub struct BandCurve {
    points: Vec<Vec3>,
}

impl BandCurve {
    pub fn new(points: Vec<Vec3>) -> Self {
        let mut this = Self { points };
        this.points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
        this
    }

    pub fn evaluate(&self, x_in: f32) -> f32 {
        if self.points.is_empty() {
            return x_in;
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

        let linear_t = (x_in - p0.x) / (p1.x - p0.x);
        let t = if p1.z.abs() < 1e-3 {
            linear_t
        } else {
            let curvature = -p1.z * 6.;
            ((curvature * linear_t).exp() - 1.) / (curvature.exp() - 1.)
        };

        p0.y + t * (p1.y - p0.y)
    }
}

impl AudioNode for MultibandCompressor {
    type Configuration = EmptyConfig;

    fn info(&self, _configuration: &Self::Configuration) -> Result<AudioNodeInfo, NodeError> {
        Ok(AudioNodeInfo::new()
            .debug_name("multiband_compressor")
            .channel_config(ChannelConfig::new(ChannelCount::STEREO, ChannelCount::STEREO)))
    }

    fn construct_processor(&self, _configuration: &Self::Configuration, cx: ConstructProcessorContext) -> Result<impl AudioNodeProcessor, NodeError> {
        Ok(MultibandCompressorProcessor {
            params: self.clone(),
            processors: ChannelProcessors::new(cx.stream_info.sample_rate, self),
        })
    }
}

pub struct MultibandCompressorProcessor {
    params: MultibandCompressor,
    processors: ChannelProcessors,
}

#[derive(Debug, Clone)]
struct ChannelProcessors {
    low_cut: [BiquadFilter; 2],
    crossover: [ThreeBandCrossover; 2],
    low_comp: [BandCompressorState; 2],
    mid_comp: [BandCompressorState; 2],
    high_comp: [BandCompressorState; 2],
    master_comp: [BandCompressorState; 2],
}

impl ChannelProcessors {
    fn new(sample_rate: NonZeroU32, params: &MultibandCompressor) -> Self {
        ChannelProcessors {
            low_cut: [BiquadFilter::new(sample_rate, params.freq_low_cut, false); 2],
            crossover: [ThreeBandCrossover::new(sample_rate, params.freq_low_cutoff, params.freq_high_cutoff); 2],
            low_comp: array::repeat(BandCompressorState::new(sample_rate, &params.low)),
            mid_comp: array::repeat(BandCompressorState::new(sample_rate, &params.mid)),
            high_comp: array::repeat(BandCompressorState::new(sample_rate, &params.high)),
            master_comp: array::repeat(BandCompressorState::new(sample_rate, &params.master)),
        }
    }
}

impl AudioNodeProcessor for MultibandCompressorProcessor {
    fn events(&mut self, info: &ProcInfo, events: &mut ProcEvents, _extra: &mut ProcExtra) {
        for patch in events.drain_patches::<MultibandCompressor>() {
            Patch::apply(&mut self.params, patch);
            self.processors = ChannelProcessors::new(info.sample_rate, &self.params);
        }
    }

    fn process(&mut self, info: &ProcInfo, ProcBuffers { inputs, outputs }: ProcBuffers, extra: &mut ProcExtra) -> ProcessStatus {
        if info.in_silence_mask.all_channels_silent(inputs.len()) {
            return ProcessStatus::ClearAllOutputs
        }

        let (&[input_l, input_r], [output_l, output_r]) = (inputs, outputs) else {
            extra.logger.try_error("Inputs and outputs must be stereo").unwrap();
            return ProcessStatus::ClearAllOutputs
        };

        let proc = &mut self.processors;
        for i in 0..info.frames {
            fn separate(l: f32, r: f32, sep: f32) -> [f32; 2] {
                let mid = 0.5 * (l + r);
                let side = 0.5 * (l - r);

                let side_gain = if sep < 0. { 1. + sep } else { 1. + sep * 2. };

                let adjusted_side = side * side_gain;
                [mid + adjusted_side, mid - adjusted_side]
            }

            let [l, r] = [proc.low_cut[0].process(input_l[i]), proc.low_cut[1].process(input_r[i])];
            let [low_l, mid_l, high_l] = proc.crossover[0].process(l);
            let [low_r, mid_r, high_r] = proc.crossover[1].process(r);

            let [low_l, low_r] = separate(
                proc.low_comp[0].process(low_l),
                proc.low_comp[1].process(low_r),
                self.params.low_stereo_separation,
            );
            let [mid_l, mid_r] = separate(
                proc.mid_comp[0].process(mid_l),
                proc.mid_comp[1].process(mid_r),
                self.params.mid_stereo_separation,
            );
            let [high_l, high_r] = separate(
                proc.high_comp[0].process(high_l),
                proc.high_comp[1].process(high_r),
                self.params.high_stereo_separation,
            );
            let [master_l, master_r] = separate(
                proc.master_comp[0].process(low_l + mid_l + high_l),
                proc.master_comp[1].process(low_r + mid_r + high_r),
                self.params.master_stereo_separation,
            );

            output_l[i] = master_l;
            output_r[i] = master_r;
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
struct BandCompressorState {
    curve: BandCurve,
    envelope: f32,
    attack_coeff: f32,
    release_coeff: f32,
    sustain_samples: usize,
    hold_counter: usize,
    pre_gain: f32,
    post_gain: f32,
}

impl BandCompressorState {
    fn new(sample_rate: NonZeroU32, params: &BandCompressor) -> Self {
        let rate = sample_rate.get() as f32;
        let sustain_samples = (params.sustain_ms * 0.001 * rate).round_ties_even() as usize;
        Self {
            curve: params.curve.clone(),
            envelope: 0.,
            attack_coeff: (-1. / (params.attack_ms * 0.001 * rate)).exp(),
            release_coeff: (-1. / (params.release_ms * 0.001 * rate)).exp(),
            sustain_samples,
            hold_counter: 0,
            pre_gain: params.pre_gain.amp(),
            post_gain: params.post_gain.amp(),
        }
    }

    #[inline]
    fn process(&mut self, sample: f32) -> f32 {
        let driven_sample = sample * self.pre_gain;
        let abs_sample = driven_sample.abs();

        if abs_sample >= self.envelope {
            self.envelope = abs_sample + self.attack_coeff * (self.envelope - abs_sample);
            self.hold_counter = self.sustain_samples;
        } else if self.hold_counter > 0 {
            self.hold_counter -= 1;
        } else {
            self.envelope = abs_sample + self.release_coeff * (self.envelope - abs_sample);
        }

        if self.envelope < 1e-6 {
            return sample * self.post_gain
        }

        let db_in = 20. * self.envelope.log10();
        let db_out = self.curve.evaluate(db_in);
        let db_gain = db_out - db_in;
        let linear_gain = 10f32.powf(db_gain * 0.05);

        driven_sample * linear_gain * self.post_gain
    }
}

pub(super) fn plugin(app: &mut App) {
    app.register_node::<MultibandCompressor>();
}
