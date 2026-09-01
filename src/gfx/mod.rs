use bevy::light::{DirectionalLightShadowMap, PointLightShadowMap};

use crate::prelude::*;

pub const LAYER_PORTAL_RESERVE: usize = 8;

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(PointLightShadowMap { size: 1024 })
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .register_required_components::<DirectionalLight, VolumetricLight>()
        .register_required_components::<PointLight, VolumetricLight>()
        .register_required_components::<SpotLight, VolumetricLight>()
        .add_observer(on_directional_light_insert_enable_shadows)
        .add_observer(on_point_light_insert_enable_shadows)
        .add_observer(on_spot_light_insert_enable_shadows);
}

pub fn on_directional_light_insert_enable_shadows(insert: On<Insert, DirectionalLight>, mut query: Query<&mut DirectionalLight>) -> Result {
    query.get_mut(insert.entity)?.shadow_maps_enabled = true;
    Ok(())
}

pub fn on_point_light_insert_enable_shadows(insert: On<Insert, PointLight>, mut query: Query<&mut PointLight>) -> Result {
    query.get_mut(insert.entity)?.shadow_maps_enabled = true;
    Ok(())
}

pub fn on_spot_light_insert_enable_shadows(insert: On<Insert, SpotLight>, mut query: Query<&mut SpotLight>) -> Result {
    query.get_mut(insert.entity)?.shadow_maps_enabled = true;
    Ok(())
}
