use std::{
    cell::RefCell,
    num::{NonZeroU32, NonZeroUsize},
    ops::Range,
    path::PathBuf,
};

use bevy::{
    asset::AssetPath,
    ecs::{define_label, intern::Interned},
};
use bevy_seedling::firewheel::{
    collector::ArcGc,
    sample_resource::{SampleResource, SampleResourceInfo},
};
use smallvec::SmallVec;

use crate::prelude::*;

define_label!(
    SoundtrackLabel,
    SOUNDTRACK_LABEL_INTERNER,
    extra_methods: {
        fn path(&self) -> &str;
    },
    extra_methods_impl: {
        fn path(&self) -> &str {
            SoundtrackLabel::path(&**self)
        }
    }
);

#[macro_export]
macro_rules! soundtrack_label {
    ($(#[$attr:meta])* $vis:vis enum $name:ident {
        $(#[path = $entry_path:expr] $entry_name:ident,)*
    }) => {
        $(#[$attr])*
        #[derive(Debug, Eq)]
        $vis enum $name {
            $($entry_name,)*
        }

        impl ::std::cmp::PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                ::std::cmp::PartialEq::eq(self.path(), other.path())
            }
        }

        impl ::std::hash::Hash for $name {
            #[inline]
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                self.path().hash(state)
            }
        }

        impl crate::soundtrack::SoundtrackLabel for $name {
            #[inline]
            fn path(&self) -> &str {
                match self {
                    $(Self::$entry_name => $entry_path,)*
                }
            }

            #[inline]
            fn dyn_clone(&self) -> ::std::boxed::Box<dyn crate::soundtrack::SoundtrackLabel> {
                todo!()
            }
        }
    };
}

impl SoundtrackLabel for String {
    #[inline]
    fn path(&self) -> &str {
        self
    }

    #[inline]
    fn dyn_clone(&self) -> Box<dyn SoundtrackLabel> {
        Box::new(self.clone())
    }
}

#[derive(TypePath, Asset, Clone)]
pub struct Soundtrack {
    #[dependency]
    pub entries: HashMap<Interned<dyn SoundtrackLabel>, Handle<AudioSample>>,
}

pub struct SoundtrackEntry {
    source: ArcGc<dyn SampleResource + Send + Sync>,
    original_sample_rate: NonZeroU32,
    loop_marker: u64,
}

impl SoundtrackEntry {
    pub fn sample_rate_factor(&self) -> Option<f64> {
        match self.source.sample_rate() {
            None => None,
            Some(sample_rate) if sample_rate == self.original_sample_rate => None,
            Some(sample_rate) => {
                Some(if sample_rate != self.original_sample_rate { sample_rate.get() as f64 / self.original_sample_rate.get() as f64 } else { 1. })
            }
        }
    }

    pub fn apply_sample_rate_factor(&self, number: u64) -> u64 {
        match self.sample_rate_factor() {
            None => number,
            Some(factor) => (number as f64 * factor).round_ties_even() as u64,
        }
    }
}

impl SampleResourceInfo for SoundtrackEntry {
    fn num_channels(&self) -> NonZeroUsize {
        self.source.num_channels()
    }

    fn len_frames(&self) -> u64 {
        self.apply_sample_rate_factor(self.loop_marker)
    }

    fn sample_rate(&self) -> Option<NonZeroU32> {
        self.source.sample_rate()
    }
}

impl SampleResource for SoundtrackEntry {
    fn fill_buffers(&self, out_buffer: &mut [&mut [f32]], mut out_buffer_range: Range<usize>, mut start_frame: u64) -> usize {
        thread_local! {
            static TAIL_BUFFER: RefCell<Vec<[f32; 1024]>> = const { RefCell::new(Vec::new()) };
        }

        let channels = self.source.num_channels().get();
        let loop_marker = self.len_frames();
        let actual_len = self.source.len_frames();

        let mut written = 0;
        while !out_buffer_range.is_empty() {
            if start_frame >= actual_len {
                // Short tracks, do not loop just yet. Zero out buffers.
                for i in 0..channels {
                    out_buffer[i][out_buffer_range.clone()].fill(0.);
                }
                written += out_buffer_range.len();
                return written
            } else if actual_len > loop_marker && start_frame < actual_len - loop_marker {
                // Long tracks, copy tail to start.
                let len = TAIL_BUFFER.with_borrow_mut(|tail_buffers| {
                    tail_buffers.resize(channels.max(tail_buffers.len()), [0.; 1024]);

                    let mut refs = SmallVec::<[&mut [f32]; 32]>::with_capacity(channels);
                    for buf in tail_buffers {
                        refs.push(&mut buf[..]);
                    }

                    let tail_len = self
                        .source
                        .fill_buffers(&mut refs, 0..out_buffer_range.len().min(1024), loop_marker + start_frame);
                    let len = self.source.fill_buffers(
                        out_buffer,
                        out_buffer_range.start..out_buffer_range.end.min(out_buffer_range.start + tail_len),
                        start_frame,
                    );

                    let processable_len = tail_len.min(len);
                    for channel in 0..channels {
                        let buffer = &mut *out_buffer[channel];
                        let tail_buffer = &mut *refs[channel];
                        for i in 0..processable_len {
                            buffer[i + out_buffer_range.start] += tail_buffer[i];
                        }
                    }

                    processable_len
                });
                if len == 0 {
                    return written
                } else {
                    written += len;
                    out_buffer_range.start += len;
                    start_frame += len as u64;
                }
            } else {
                // Normal case.
                let len = self.source.fill_buffers(out_buffer, out_buffer_range.clone(), start_frame);
                if len == 0 {
                    return written
                } else {
                    written += len;
                    out_buffer_range.start += len;
                    start_frame += len as u64;
                }
            }
        }

        written
    }
}

#[derive(Reflect, Debug, Clone, Copy)]
#[reflect(Debug, Clone)]
pub struct SoundtrackLoader;
impl AssetLoader for SoundtrackLoader {
    type Asset = Soundtrack;
    type Settings = ();
    type Error = BevyError;

    async fn load(&self, reader: &mut dyn Reader, _: &Self::Settings, load_context: &mut LoadContext<'_>) -> Result<Self::Asset, Self::Error> {
        #[derive(Deserialize)]
        struct File {
            loop_marker: u64,
            entries: Vec<PathBuf>,
        }

        let path = load_context.path().clone();
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes).await?;

        let mut soundtrack = Soundtrack { entries: HashMap::new() };
        let file = ron::de::from_bytes::<File>(&bytes)?;
        for entry in file.entries {
            let src = load_context
                .load_builder()
                .load_value::<AudioSample>(path.resolve_embed(&AssetPath::from_path(&entry)))
                .await?
                .take();

            let label = entry.into_string().map_err(|e| format!("Invalid soundtrack path '{}'", e.display()))?;
            let source = src.get();
            let original_sample_rate = src.original_sample_rate();

            soundtrack.entries.insert(
                label.intern(),
                load_context.add_labeled_asset(
                    label,
                    AudioSample::new(
                        SoundtrackEntry {
                            source,
                            original_sample_rate,
                            loop_marker: file.loop_marker,
                        },
                        original_sample_rate,
                    ),
                ),
            );
        }

        Ok(soundtrack)
    }

    fn extensions(&self) -> &[&str] {
        &[".mus.ron"]
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<Soundtrack>().register_asset_loader(SoundtrackLoader);
}
