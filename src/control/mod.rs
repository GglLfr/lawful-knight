use crate::{
    camera::{DEFAULT_CAMERA_DISTANCE, PrimaryCamera},
    prelude::*,
};

mod scene;
mod surface;
pub use scene::*;
pub use surface::*;

#[derive(Component)]
pub struct Player;

#[derive(InputAction)]
#[action_output(Vec2)]
struct Move;

pub fn add_player(ready: On<Remove, ScenePending>, mut commands: Commands) {
    commands
        .spawn((
            Player,
            RigidBody::Dynamic,
            TransformExtrapolation,
            TransformHermiteEasing,
            Collider::sphere(0.5),
            TangentSpace::default(),
            Transform::from_xyz(0., 1., 0.),
            actions!(Player[(Action::<Move>::new(), DeadZone::default(), Bindings::spawn(Cardinal::wasd_keys()),)]),
        ))
        .observe(|fire: On<Fire<Move>>, mut forces: Query<(Forces, &TangentSpace)>| -> Result {
            let (mut forces, space) = forces.get_mut(fire.context)?;
            *forces.linear_velocity_mut() = space.mul_vec3((fire.value * 10.).extend(0.).into());
            Ok(())
        })
        .observe(|stop: On<Complete<Move>>, mut forces: Query<Forces>| -> Result {
            let mut forces = forces.get_mut(stop.context)?;
            *forces.linear_velocity_mut() = Vec3::ZERO;
            Ok(())
        });

    commands.entity(ready.observer()).despawn();
}

#[derive(Reflect, Component, Debug, Clone, Copy)]
#[reflect(Component, Debug, Clone)]
pub struct CameraState {
    pub up: Vector3,
    pub forward: Vector3,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            up: Vector3::Y,
            forward: Vector3::NEG_Z,
        }
    }
}

pub fn move_camera(
    time: Res<Time>,
    camera: Single<(&mut Transform, &mut CameraState), (With<PrimaryCamera>, Without<Player>)>,
    player: Single<(&Transform, &TangentSpace), With<Player>>,
) {
    let (mut camera_trns, mut camera_state) = camera.into_inner();
    let (player_trns, player_tangent_space) = player.into_inner();

    camera_state.up.smooth_nudge(&player_tangent_space.up(), f32::ln(5.), time.delta_secs());
    camera_state
        .forward
        .smooth_nudge(&player_tangent_space.forward(), f32::ln(5.), time.delta_secs());

    *camera_trns = Transform::from_translation(player_trns.translation - camera_state.forward * DEFAULT_CAMERA_DISTANCE)
        .looking_at(player_trns.translation, camera_state.up);
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((scene::plugin, surface::plugin))
        .register_required_components::<PrimaryCamera, CameraState>()
        // Gravity isn't uniform in this game. Simulate using linear acceleration directly.
        .insert_resource(Gravity(Vec3::ZERO))
        .add_input_context::<Player>()
        .add_observer(add_player)
        .add_systems(Update, move_camera);
}
