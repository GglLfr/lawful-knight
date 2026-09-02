use crate::prelude::*;

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

#[derive(Reflect, Component, Debug, Clone, Copy)]
#[reflect(Component, Debug, Clone)]
#[component(immutable)]
#[relationship(relationship_target = Projections)]
pub struct ProjectedOn(pub Entity);

#[derive(Reflect, Component, Debug, Clone)]
#[reflect(Component, Debug, Clone)]
#[relationship_target(relationship = ProjectedOn)]
pub struct Projections(Vec<Entity>);
impl<'a> IntoIterator for &'a Projections {
    type Item = &'a Entity;
    type IntoIter = std::slice::Iter<'a, Entity>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

pub(super) fn plugin(app: &mut App) {
    app.insert_skein_preset("default", ControlLayers::with_default())
        .insert_skein_preset("surface", ControlLayers::control_surface());
}
