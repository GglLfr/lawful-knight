use bevy::{app::SceneSpawnerSystems, tasks::ComputeTaskPool};

use crate::{
    camera::ClipMaterial,
    control::{ConstraintSurface, ConstraintSurfaceMarker, ControlLayers},
    prelude::*,
};

#[derive(Reflect, Component, Default, Clone, Copy)]
#[reflect(Component, Default, Clone)]
pub struct NoSceneCollider;

pub fn insert_scene_colliders(
    commands: ParallelCommands,
    pendings: Query<(Entity, &WorldInstance), With<ScenePending>>,
    spawner: Res<WorldInstanceSpawner>,
    meshes: Res<Assets<Mesh>>,
    query: Query<(Option<&Mesh3d>, Has<NoSceneCollider>, Has<ConstraintSurfaceMarker>, Option<&Children>)>,
) -> Result {
    fn handle<'scope, 'env>(
        scope: &'scope bevy::tasks::Scope<'scope, 'env, Result>,
        commands: &'env ParallelCommands,
        meshes: &'env Assets<Mesh>,
        entity: Entity,
        mut is_skip: bool,
        mut is_constraint: bool,
        query: &'env Query<(Option<&Mesh3d>, Has<NoSceneCollider>, Has<ConstraintSurfaceMarker>, Option<&Children>)>,
    ) {
        let Ok((mesh, skip, has_constraint, children)) = query.get(entity) else { return };
        is_skip |= skip;
        is_constraint |= has_constraint;

        if has_constraint {
            commands.command_scope(|mut commands| {
                commands.entity(entity).remove::<ConstraintSurfaceMarker>();
            });
        }

        if !is_skip
            && let Some(mesh) = mesh
            && let Some(mesh) = meshes.get(mesh.id())
        {
            scope.spawn(async move {
                if is_constraint {
                    let collider = Collider::trimesh_from_mesh(mesh).ok_or("Couldn't create constraint surface mesh")?;
                    commands.command_scope(|mut commands| {
                        commands
                            .entity(entity)
                            .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>, MeshMaterial3d<ClipMaterial>)>()
                            .insert((collider, ConstraintSurface, ControlLayers::constraint_surface()));
                    });
                } else {
                    let collider = Collider::convex_decomposition_from_mesh_with_config(mesh, &VhacdParameters {
                        concavity: 0.001,
                        ..default()
                    })
                    .ok_or("Couldn't create terrain mesh")?;
                    commands.command_scope(|mut commands| {
                        commands.entity(entity).insert((collider, ControlLayers::with_default()));
                    });
                }
                Ok(())
            });
        }

        if let Some(children) = children {
            for &child in children {
                handle(scope, commands, meshes, child, is_skip, is_constraint, &query);
            }
        }
    }

    for (scene_entity, scene_id) in &pendings {
        if !spawner.instance_is_ready(**scene_id) {
            continue
        }

        let results = ComputeTaskPool::get().scope(|scope| handle(scope, &commands, &meshes, scene_entity, false, false, &query));
        commands.command_scope(|mut commands| {
            commands.entity(scene_entity).remove::<ScenePending>();
        });

        let errors = results.into_iter().filter_map(|result| result.err()).fold(String::new(), |mut str, e| {
            _ = write!(&mut str, "{e}\n");
            str
        });

        if !errors.is_empty() {
            return Err(errors.into())
        }
    }
    Ok(())
}

#[derive(Reflect, Component, Default, Clone, Copy)]
#[reflect(Component, Default, Clone)]
#[component(storage = "SparseSet")]
pub struct ScenePending;

pub(super) fn plugin(app: &mut App) {
    app.register_required_components::<WorldInstance, ScenePending>()
        .add_systems(SpawnScene, insert_scene_colliders.after(SceneSpawnerSystems::WorldInstanceSpawn));
}
