use crate::prelude::*;

#[derive(Reflect, Component, ExtractComponent, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(
    IsDefaultUiCamera,
    GameCamera,
    Bloom {
        intensity: 0.2,
        ..Bloom::NATURAL
    },
    TemporalAntiAliasing,
    ContrastAdaptiveSharpening,
    DebandDither::Enabled
)]
pub struct PrimaryCamera;

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(
    Camera3d,
    Hdr,
    Msaa::Off,
    ShadowFilteringMethod::Temporal,
    VolumetricFog {
        ambient_intensity: 0.,
        jitter: 0.64,
        ..default()
    },
)]
pub struct GameCamera;
