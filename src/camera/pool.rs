use crate::{
    camera::{GameCamera, PrimaryCamera},
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.configure_sets(
        PostUpdate,
        // Extend `CameraUpdateSystems` so it does 1) update the primary camera first, 2) pool additional cameras, and 3) what it does normally.
        (
            PooledCameraSystems::Prepare,
            PooledCameraSystems::Obtain,
            PooledCameraSystems::UpdateImages,
        )
            .chain()
            .in_set(CameraUpdateSystems)
            .before(camera_system)
            .before(VisibilitySystems::UpdateFrusta)
            .before(VisibilitySystems::VisibilityPropagate),
    )
    .add_systems(Startup, init_camera_pool)
    .add_systems(PreUpdate, free_pooled_cameras.in_set(PooledCameraSystems::Free))
    .add_systems(
        PostUpdate,
        (
            update_primary_camera.in_set(PooledCameraSystems::Prepare),
            update_pooled_dirty_images.in_set(PooledCameraSystems::UpdateImages),
        ),
    );
}

#[derive(Reflect, Component, Debug, Default, Clone, Copy)]
#[reflect(Component, Debug, Default, Clone)]
#[require(GameCamera)]
pub struct PooledCamera;

#[derive(Reflect, SystemSet, Debug, Clone, Eq, PartialEq, Hash)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub enum PooledCameraSystems {
    Free,
    Prepare,
    Obtain,
    UpdateImages,
}

/// `camera_system` and `update_frusta` combined specifically for the primary camera only, along
/// with global transform computation. This is to ensure systems that need to pool cameras can do
/// frustum culling properly.
///
/// Yes, this means the primary camera is updated twice per frame, but oh well.
//TODO 0.19 allows system removal
pub fn update_primary_camera(
    mut pool: ResMut<CameraPool>,
    window_resized_reader: MessageReader<WindowResized>,
    window_created_reader: MessageReader<WindowCreated>,
    window_scale_factor_changed_reader: MessageReader<WindowScaleFactorChanged>,
    image_asset_event_reader: MessageReader<AssetEvent<Image>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<
        (
            Entity,
            Ref<Transform>,
            Mut<GlobalTransform>,
            &mut Frustum,
            &mut Camera,
            &RenderTarget,
            &mut Projection,
        ),
        With<PrimaryCamera>,
    >,
    mut last_size: Local<UVec2>,
) -> Result {
    // Assume `PrimaryCamera` has no parent entity.
    for (_, trns, mut global_trns, _, _, _, _) in &mut cameras {
        if trns.is_changed() {
            global_trns.set_if_neq(GlobalTransform::from(*trns));
        }
        break
    }

    camera_system(
        window_resized_reader,
        window_created_reader,
        window_scale_factor_changed_reader,
        image_asset_event_reader,
        primary_window,
        windows,
        images,
        manual_texture_views,
        cameras.transmute_lens().query(),
    )?;

    for (_, _, trns, mut frustum, camera, _, projection) in &mut cameras {
        let Some(size) = camera.physical_target_size() else { return Ok(()) };
        if replace(&mut *last_size, size) != size {
            pool.needs_resize = true;
        }

        if trns.is_changed() || projection.is_changed() {
            *frustum = projection.compute_frustum(&*trns);
        }
        break
    }

    Ok(())
}

#[derive(Resource)]
pub struct CameraPool {
    needs_resize: bool,
    image_provider: AssetHandleProvider,
    images: Vec<Handle<Image>>,
    dirty_image: usize,
    allocated: Vec<Entity>,
    free: Vec<Entity>,
}

pub fn init_camera_pool(mut commands: Commands, images: Res<Assets<Image>>) {
    commands.insert_resource(CameraPool {
        needs_resize: false,
        image_provider: images.get_handle_provider(),
        images: vec![],
        dirty_image: 0,
        allocated: vec![],
        free: vec![],
    });
}

pub fn free_pooled_cameras(pool: ResMut<CameraPool>, mut cameras: Query<&mut Camera>) {
    let pool = pool.into_inner();
    let mut iter = cameras.iter_many_mut(&pool.allocated);
    while let Some(mut camera) = iter.fetch_next() {
        camera.is_active = false;
    }

    pool.free.append(&mut pool.allocated);
}

pub fn update_pooled_dirty_images(
    pool: ResMut<CameraPool>,
    mut images: ResMut<Assets<Image>>,
    camera: Single<(&Camera, Has<Hdr>), With<PrimaryCamera>>,
) -> Result {
    let (camera, is_hdr) = camera.into_inner();
    let Some(size) = camera.physical_target_size() else { return Ok(()) };

    let pool = pool.into_inner();
    if replace(&mut pool.needs_resize, false) {
        pool.dirty_image = 0;
    }

    for handle in &pool.images[replace(&mut pool.dirty_image, pool.images.len())..] {
        let new = Image {
            asset_usage: RenderAssetUsages::RENDER_WORLD,
            ..Image::new_target_texture(
                size.x,
                size.y,
                match is_hdr {
                    false => TextureFormat::Rgba8UnormSrgb,
                    true => TextureFormat::Rgba16Float,
                },
                None,
            )
        };

        if let Some(mut image) = images.get_mut(handle) {
            *image = new;
        } else {
            images.insert(handle, new)?;
        }
    }

    Ok(())
}

macro_rules! query_impl {
    ($($name:ident : $t:ty,)*) => {
        pub type CameraPoolQuery<'w, 's> = Query<'w, 's, (Entity, Read<RenderTarget>, $(Write<$t>,)*), With<PooledCamera>>;

        pub struct PooledCameraParams<'w> {
            pub entity: Entity,
            pub image: &'w Handle<Image>,
            $(pub $name: &'w mut $t,)*
        }

        impl<'w> PooledCameraParams<'w> {
            pub fn from_item<'s>((entity, target, $($name,)*): QueryItem<'w, 's, (Entity, Read<RenderTarget>, $(Write<$t>,)*)>) -> Result<Self> {
                Ok(Self {
                    entity,
                    image: match target {
                        RenderTarget::Image(target) => &target.handle,
                        _ => Err("Pooled cameras must render to an image")?,
                    },
                    $($name: $name.into_inner(),)*
                })
            }
        }
    };
}

query_impl! {
    camera: Camera,
    projection: Projection,
    layers: RenderLayers,
}

impl CameraPool {
    pub fn needs_resize(&self) -> bool {
        self.needs_resize
    }

    pub fn obtain<T>(
        &mut self,
        commands: &mut Commands,
        query: &mut CameraPoolQuery,
        apply: impl FnOnce(&mut Commands, PooledCameraParams) -> Result<T>,
    ) -> Result<T> {
        if let Some(e) = self.free.pop() {
            self.allocated.push(e);

            let item = PooledCameraParams::from_item(query.get_mut(e)?)?;
            item.camera.is_active = true;
            apply(commands, item)
        } else {
            let mut camera = Camera::default();
            let image = self.image_provider.reserve_handle().typed_debug_checked::<Image>();
            let mut projection = Projection::default();
            let mut layers = RenderLayers::default();

            let entity = commands.spawn_empty().id();
            let result = apply(commands, PooledCameraParams {
                entity,
                image: &image,
                camera: &mut camera,
                projection: &mut projection,
                layers: &mut layers,
            })?;

            self.allocated.push(entity);
            self.images.push(image.clone());

            commands
                .entity(entity)
                .insert((PooledCamera, RenderTarget::Image(image.into()), camera, projection, layers));
            Ok(result)
        }
    }
}
