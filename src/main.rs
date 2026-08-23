pub mod prelude {
    pub use std::{
        array,
        cell::RefCell,
        cmp::Ordering,
        f32::consts::{PI, SQRT_2, TAU},
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

use crate::{
    environment::portal::PortalCollisionHooks,
    prelude::*,
    soundtrack::{SoundtrackPlayed, SoundtrackPlayer, SoundtrackState},
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
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::from(WgpuSettings {
                    features: WgpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | WgpuFeatures::CLIP_DISTANCES,
                    ..default()
                }),
                ..default()
            }),
            #[cfg(feature = "dev")]
            FpsOverlayPlugin::default(), /* .set(WindowPlugin {
                                             primary_window: Some(Window {
                                                 decorations: false,
                                                 resolution: [720; 2].into(),
                                                 ..default()
                                             }),
                                             ..default()
                                         })*/
            report_mimalloc_version,
            PhysicsPlugins::default().with_collision_hooks::<PortalCollisionHooks>(),
            //PhysicsDebugPlugin,
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
        .add_systems(Update, move_around)
        .run()
}

#[derive(Component)]
struct Shift(f32, bool, bool);

fn move_around(time: Res<Time>, mut transforms: Query<(&mut Transform, &Shift)>) {
    let t = (time.elapsed_secs() / 2.).fract();
    for (trns, mov) in &mut transforms {
        let trns = trns.into_inner();
        trns.translation.y = match (mov.1, mov.2) {
            (false, false) => mov.0 - t * 7.5,
            (false, true) => mov.0 - t * 7.5 + 7.5,
            (true, false) => mov.0 + t * 7.5,
            (true, true) => mov.0 + t * 7.5 - 7.5,
        };

        trns.scale.x = match (mov.1, mov.2) {
            (false, false) | (true, false) => t * 15.,
            (false, true) | (true, true) => (1. - t) * 15.,
        };
    }
}

fn game_init(mut commands: Commands, server: Res<AssetServer>, mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::InGame);
    commands.spawn((
        WorldAssetRoot(server.load(GltfAssetLabel::Scene(0).from_asset("zones/zone_master.gltf"))),
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexDecompositionFromMesh),
    ));

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
                move |played: On<SoundtrackPlayed>, mut commands: Commands, state: Query<&SoundtrackState>| -> Result {
                    let state = state.get(played.entity)?;
                    for (key, &sample_entity) in &state.entries {
                        use bevy::{
                            picking::hover::Hovered,
                            ui_widgets::{Activate, Button},
                        };

                        let key_name = key.path().to_string();
                        commands
                            .spawn_scene(bsn! {
                                ChildOf(root)
                                Node {
                                    padding: UiRect::all(px(16))
                                }
                                BackgroundColor(ACTIVATED)
                                Button
                                Hovered
                                Children [
                                    ~Text::new(key_name)
                                ]
                            })
                            .observe(
                                move |activated: On<Activate>,
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
                                },
                            );
                    }

                    Ok(())
                },
            );
    }
}
