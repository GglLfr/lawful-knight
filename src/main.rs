pub mod prelude {
    pub use std::{
        array,
        borrow::Cow,
        cell::RefCell,
        cmp::Ordering,
        f32::consts::{PI, SQRT_2, TAU},
        fmt::Debug,
        mem::replace,
        num::{NonZeroU32, NonZeroUsize},
        ops::{Mul, Range},
        path::PathBuf,
        ptr::addr_eq,
    };

    pub use avian3d::{physics_transform::PhysicsTransformSystems, prelude::*};
    #[cfg(feature = "dev")]
    pub use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
    pub use bevy::{
        anti_alias::{contrast_adaptive_sharpening::ContrastAdaptiveSharpening, taa::TemporalAntiAliasing},
        asset::{AsAssetId, AssetHandleProvider, AssetLoader, AssetPath, LoadContext, ReflectAsset, RenderAssetUsages, io::Reader},
        camera::{
            CameraProjection, CameraUpdateSystems, Hdr, RenderTarget, SubCameraView,
            primitives::{Aabb, Frustum},
            visibility::{NoAutoAabb, RenderLayers, VisibilitySystems},
        },
        core_pipeline::{
            core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
            prepass::{AlphaMask3dPrepass, Opaque3dPrepass},
            tonemapping::DebandDither,
        },
        ecs::{
            component::Mutable,
            define_label,
            entity::{EntityHashMap, EntityHashSet},
            intern::Interned,
            lifecycle::HookContext,
            query::{QueryData, QueryItem, ROQueryItem},
            system::{
                ReadOnlySystemParam, SystemParam, SystemParamItem,
                lifetimeless::{Read, SRes, Write},
            },
            template::TemplateContext,
            world::DeferredWorld,
        },
        light::{FogVolume, ShadowFilteringMethod, VolumetricFog, VolumetricLight},
        math::{Affine3A, Curve},
        mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
        pbr::{
            DrawMaterial, DrawPrepass, ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, Shadow, Transmissive3d,
        },
        platform::collections::{HashMap, hash_map::Entry},
        post_process::bloom::Bloom,
        prelude::*,
        render::{
            Extract, Render, RenderApp, RenderPlugin, RenderStartup, RenderSystems,
            camera::camera_system,
            extract_component::{ExtractComponent, ExtractComponentPlugin},
            render_phase::{Draw, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderCommandState, TrackedRenderPass},
            render_resource::{
                AsBindGroup, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BufferDescriptor, BufferUsages, DynamicUniformBuffer,
                PipelineCache, RenderPipelineDescriptor, ShaderStages, ShaderType, SpecializedMeshPipelineError, TextureFormat,
                binding_types::uniform_buffer,
            },
            renderer::{RenderDevice, RenderQueue},
            settings::{RenderCreation, WgpuFeatures, WgpuSettings},
            sync_world::RenderEntity,
            view::ViewTarget,
        },
        shader::{ShaderDefVal, ShaderRef},
        utils::Parallel,
        window::{PrimaryWindow, WindowCreated, WindowResized, WindowScaleFactorChanged},
    };
    pub use bevy_enhanced_input::prelude::{self::*, Cancel, Press, Release};
    pub use bevy_framepace::FramepacePlugin;
    pub use bevy_seedling::{
        firewheel::{
            channel_config::ChannelConfig,
            collector::ArcGc,
            core as firewheel_core,
            diff::{Diff, Patch},
            event::ProcEvents,
            node::{
                AudioNode, AudioNodeInfo, AudioNodeProcessor, ConstructProcessorContext, EmptyConfig, NodeError, ProcBuffers, ProcExtra, ProcInfo,
                ProcessStatus,
            },
            sample_resource::{SampleResource, SampleResourceInfo},
        },
        pool::CompletionReason,
        prelude::*,
    };
    pub use bevy_skein::{SkeinAppExt as _, SkeinPlugin};
    pub use bevy_sprinkles::prelude::*;
    pub use bevy_transform_interpolation::{RotationEasingState, ScaleEasingState, TranslationEasingState, prelude::*};
    pub use mimalloc_redirect::MiMalloc;
    pub use ron;
    pub use serde::Deserialize;
    pub use smallvec::SmallVec;
}

use bevy_seedling::sample::AudioLoaderConfig;
use symphonia_adapter_libopus::OpusDecoder;

use crate::{
    //environment::portal::PortalCollisionHooks,
    prelude::*,
    soundtrack::{SoundtrackPlay, SoundtrackPlayer, SoundtrackState},
};

pub mod camera;
pub mod control;
pub mod environment;
pub mod gfx;
pub mod math;
pub mod soundtrack;

#[derive(Reflect, States, Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[reflect(State, Debug, Default, Clone, PartialEq, PartialOrd, Hash)]
pub enum GameState {
    #[default]
    Init,
    Menu,
    Load,
    InGame,
}

#[global_allocator]
static ALLOC: MiMalloc = MiMalloc;

fn report_mimalloc_version(_: &mut App) {
    info!("Using MiMalloc {}.", MiMalloc::get_version());
}

fn main() -> AppExit {
    let mut config = AudioLoaderConfig::default();
    config.register_codec(["ogg"], |registry, _| {
        registry.register_audio_decoder::<OpusDecoder>();
    });

    App::new()
        .insert_resource(config)
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::from(WgpuSettings {
                    features: WgpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | WgpuFeatures::CLIP_DISTANCES,
                    ..default()
                }),
                ..default()
            }),
            report_mimalloc_version,
            #[cfg(feature = "dev")]
            FpsOverlayPlugin::default(),
            FramepacePlugin,
            PhysicsPlugins::default(), //.with_collision_hooks::<PortalCollisionHooks>(),
            #[cfg(feature = "dev")]
            PhysicsDebugPlugin,
            EnhancedInputPlugin,
            SeedlingPlugins,
            SkeinPlugin {
                handle_brp: cfg!(feature = "dev"),
            },
            SprinklesPlugin,
            (camera::plugin, control::plugin, environment::plugin, gfx::plugin, soundtrack::plugin),
        ))
        .init_state::<GameState>()
        .add_systems(Startup, game_init)
        .run()
}

fn game_init(mut commands: Commands, server: Res<AssetServer>, mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::InGame);
    commands.spawn(WorldAssetRoot(server.load(GltfAssetLabel::Scene(0).from_asset("zones/zone_midway.gltf"))));

    #[cfg(feature = "dev")]
    {
        const ACTIVATED: Color = Color::srgba(0.2, 0.8, 0.3, 0.5);
        const UNACTIVATED: Color = Color::srgba(0., 0., 0., 0.5);

        let root = commands
            .spawn_scene(bsn! {
                Node {
                    flex_direction: FlexDirection::Column
                }
            })
            .id();

        commands
            .spawn(SoundtrackPlayer(server.load("soundtracks/midway/behind_the_mirror.mus.ron")))
            .observe(
                move |played: On<SoundtrackPlay>, mut commands: Commands, state: Query<&SoundtrackState>| -> Result {
                    use std::collections::BTreeMap;

                    use crate::soundtrack::SoundtrackLabelInfo;

                    commands.entity(root).despawn_children();

                    let state = state.get(played.entity)?;
                    let mut map = BTreeMap::new();
                    for (key, &sample_entity) in &state.entries {
                        map.insert(key.path(), sample_entity);
                    }

                    for (key, sample_entity) in map {
                        use bevy::{
                            picking::hover::Hovered,
                            ui_widgets::{Activate, Button},
                        };

                        let key_name = key.to_string();
                        commands.spawn_scene(bsn! {
                            ChildOf(root)
                            Node {
                                padding: UiRect::all(px(16))
                            }
                            BackgroundColor(ACTIVATED)
                            Button
                            Hovered
                            on(move |activated: On<Activate>,
                                  mut query: Query<&mut BackgroundColor>,
                                  effects: Query<&SampleEffects>,
                                  mut volume: Query<&mut VolumeNode>|
                                  -> Result {
                                let mut bg = query.get_mut(activated.entity)?;
                                let effects = effects.get(sample_entity)?;

                                let volume = volume.get_effect_mut(effects)?.into_inner();
                                volume.volume = match volume.volume.linear() {
                                    0. => {
                                        bg.0 = ACTIVATED;
                                        Volume::Linear(1.)
                                    }
                                    _ => {
                                        bg.0 = UNACTIVATED;
                                        Volume::Linear(0.)
                                    }
                                };

                                Ok(())
                            })
                            Children [
                                Text::new(key_name)
                            ]
                        });
                    }

                    Ok(())
                },
            );
    }
}
