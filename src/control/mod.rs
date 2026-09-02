use crate::prelude::*;

mod ground;
pub use ground::*;

#[derive(Reflect, PhysicsLayer, Debug, Default, Clone, Copy)]
#[reflect(Debug, Default, Clone)]
pub enum ControlLayers {
    // NOTE: Do not rearrange these, ever!
    #[default]
    Default,
    Surface,
}

impl ControlLayers {
    pub fn with_default() -> CollisionLayers {
        CollisionLayers::new([Self::Default], [Self::Default])
    }

    pub fn control_surface() -> CollisionLayers {
        CollisionLayers::new([Self::Surface], [] as [Self; 0])
    }
}

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ControlSurface;

pub(super) fn plugin(app: &mut App) {
    app.insert_skein_preset("default", ControlLayers::with_default())
        .insert_skein_preset("surface", ControlLayers::control_surface());
}
