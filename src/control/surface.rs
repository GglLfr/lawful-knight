use avian3d::{
    collider_tree::ColliderTrees,
    parry::math::{Matrix, Pose3},
};

use crate::prelude::*;

#[derive(Reflect, PhysicsLayer, Debug, Default, Clone, Copy)]
#[reflect(Debug, Default, Clone)]
pub enum ControlLayers {
    // NOTE: Do not rearrange these, ever!
    #[default]
    Default,
    Constraint,
}

impl ControlLayers {
    pub fn with_default() -> CollisionLayers {
        CollisionLayers::new([Self::Default], [Self::Default])
    }

    pub fn constraint_surface() -> CollisionLayers {
        CollisionLayers::new([Self::Constraint], [] as [Self; 0])
    }
}

// Workaround for Skein, because the components aren't actually inserted to the mesh entity itself.
// Major headache.
#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[component(storage = "SparseSet")]
pub struct ConstraintSurfaceMarker;

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ConstraintSurface;

#[derive(Reflect, Component, Debug, Default, Clone, Copy, Deref, DerefMut)]
#[reflect(Component, Debug, Default, Clone)]
pub struct TangentSpace(pub Matrix3);
impl TangentSpace {
    pub fn right(&self) -> Vector3 {
        self.mul_vec3(Vector3::X)
    }

    pub fn up(&self) -> Vector3 {
        self.mul_vec3(Vector3::Y)
    }

    pub fn back(&self) -> Vector3 {
        self.mul_vec3(Vector3::Z)
    }

    pub fn left(&self) -> Vector3 {
        self.mul_vec3(Vector3::NEG_X)
    }

    pub fn down(&self) -> Vector3 {
        self.mul_vec3(Vector3::NEG_Y)
    }

    pub fn forward(&self) -> Vector3 {
        self.mul_vec3(Vector3::NEG_Z)
    }
}

pub fn project_point_and_get_normal(
    colliders: &Query<(&Position, &Rotation, &Collider)>,
    collider_trees: &ColliderTrees,
    point: Vector3,
    filter: &SpatialQueryFilter,
    predicate: impl Fn(Entity) -> bool,
) -> Option<(Entity, Vector3, Vector3)> {
    let mut closest_dst2 = Scalar::INFINITY;
    let mut closest = None;

    collider_trees.iter_trees().for_each(|tree| {
        tree.squared_distance_traverse_closest(point, closest_dst2, |proxy_id| {
            let proxy = tree.get_proxy(proxy_id).unwrap();
            if !filter.test(proxy.collider, proxy.layers) || !predicate(proxy.collider) {
                return closest_dst2
            }

            let Ok((&position, &rotation, collider)) = colliders.get(proxy.collider) else { return closest_dst2 };
            let shape = collider.shape_scaled();
            let pose = Pose3::from_parts(*position, *rotation);

            let (local_projection, feature) = shape.project_local_point_and_get_feature(pose.inverse_transform_point(point));
            let projected_point = pose.transform_point(local_projection.point);

            let dst2 = (projected_point - point).length_squared();
            if dst2 < closest_dst2 {
                let local_normal = shape.feature_normal_at_point(feature, local_projection.point).unwrap_or_else(|| {
                    warn!(
                        "Constraint surface {} didn't have a local normal vector, using Vector3::Z instead. This is definitely incorrect behavior.",
                        proxy.collider
                    );
                    Vector3::Z
                });

                closest_dst2 = dst2;
                closest = Some((proxy.collider, projected_point, pose.transform_vector(local_normal)));
            }

            dst2
        });
    });

    closest
}

pub fn constrain_bodies(
    mut entities: Query<(&mut Position, &mut TangentSpace), Without<ConstraintSurface>>,
    surfaces: Query<&Rotation, With<ConstraintSurface>>,
    colliders: Query<(&Position, &Rotation, &Collider), With<ConstraintSurface>>,
    collider_trees: Res<ColliderTrees>,
) {
    let filter = SpatialQueryFilter::default();
    let colliders = colliders.transmute_lens_inner();
    let colliders = colliders.query_inner();
    let collider_trees = collider_trees.into_inner();

    entities.par_iter_mut().for_each(move |(mut position, mut tangent)| {
        let Some((surface_entity, point_at_surface, normal_at_surface)) =
                // TODO: Indiscriminately allowing any surfaces may make the entities accidentally switch to other surface groups.
                project_point_and_get_normal(&colliders, &collider_trees, **position, &filter, |_| true)
            else {
                return
            };
        let Ok(&surface_rotation) = surfaces.get(surface_entity) else { return };

        // Local +X, which will be used as the axis for horizontal movement. The contract is that the
        // surface may be deformed only by rotating cross sections along the X axis. Therefore, +X is
        // uniform across the surface, but +Y and +Z may change.
        let local_x = surface_rotation * Vector3::X;
        // Local +Z, which is where the camera should be positioned.
        let local_z = normal_at_surface * normal_at_surface.dot(surface_rotation * Vector3::Z).signum();
        // Local +Y, which will be used as the axis for vertical movement.
        let local_y = local_z.cross(local_x);

        **position = point_at_surface;
        *tangent = TangentSpace(Matrix::from_cols(local_x, local_y, local_z));
    });
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        FixedPostUpdate,
        constrain_bodies
            .in_set(PhysicsSystems::Writeback)
            .before(PhysicsTransformSystems::PositionToTransform),
    );
}
