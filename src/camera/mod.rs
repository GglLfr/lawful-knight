use crate::{gfx::LAYER_PORTAL_RESERVE, prelude::*};

mod clip;
mod def;
mod pool;
pub use clip::*;
pub use def::*;
pub use pool::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((ExtractComponentPlugin::<PrimaryCamera>::default(), clip::plugin, pool::plugin))
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(GlobalAmbientLight::NONE)
        .add_systems(Startup, spawn_camera);

    #[cfg(feature = "dev")]
    {
        use bevy::camera_controller::pan_camera::{PanCamera, PanCameraPlugin};

        app.add_plugins(PanCameraPlugin)
            .register_required_components_with::<PrimaryCamera, PanCamera>(|| PanCamera { pan_speed: 10., ..default() });
    }
}

pub const DEFAULT_CAMERA_DISTANCE: f32 = 20.;

// TODO turn this into BSN once FogVolume implements FromTemplate
pub fn camera_fog() -> impl Bundle {
    (
        FogVolume {
            fog_color: Color::linear_rgb(0.58, 0.49, 0.76),
            density_factor: 0.06,
            density_texture: None,
            density_texture_offset: Vec3::ZERO,
            absorption: 0.,
            scattering: 0.64,
            scattering_asymmetry: 0.75,
            light_tint: Color::WHITE,
            light_intensity: 1.,
        },
        Transform::from_scale(Vec3::splat(60.)),
    )
}

pub fn primary_camera() -> impl Bundle {
    (
        PrimaryCamera,
        RenderLayers::from_iter([0, LAYER_PORTAL_RESERVE]),
        Transform::from_xyz(0., 0., DEFAULT_CAMERA_DISTANCE).looking_at(Vec3::ZERO, Vec3::Y),
        children![camera_fog(),],
    )
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(primary_camera());
}
