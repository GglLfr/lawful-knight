use crate::prelude::*;

mod surface;
pub use surface::*;

#[derive(Component)]
struct Player;

#[derive(InputAction)]
#[action_output(Vec2)]
struct Move;

pub fn add_player(mut commands: Commands) {
    commands.spawn((
        Player,
        RigidBody::Dynamic,
        Collider::sphere(0.5),
        Transform::from_xyz(0., 1., 0.),
        actions!(
            Player[(
                Action::<Move>::new(),
                DeadZone::default(),
                SmoothNudge::default(),
                DeltaScale::default(),
                Bindings::spawn(Cardinal::wasd_keys()),
            )]
        ),
    ));
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(surface::plugin)
        // Gravity isn't uniform in this game. Simulate using linear acceleration directly.
        .insert_resource(Gravity(Vec3::ZERO))
        .add_input_context::<Player>()
        .add_systems(Startup, add_player);
}
