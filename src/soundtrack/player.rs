use crate::{
    prelude::*,
    soundtrack::{Soundtrack, SoundtrackLabel, effects::MultibandCompressor},
};

#[derive(Reflect, Component, FromTemplate, Default, Debug, Clone)]
#[reflect(Component, Debug, Clone)]
#[require(SoundtrackState)]
pub struct SoundtrackPlayer(pub Handle<Soundtrack>);
impl AsAssetId for SoundtrackPlayer {
    type Asset = Soundtrack;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.0.id()
    }
}

#[derive(Reflect, Component, Default, Debug, Clone)]
#[reflect(opaque, Component, Default, Debug, Clone)]
pub struct SoundtrackState {
    pub entries: HashMap<Interned<dyn SoundtrackLabel>, Entity>,
}

#[derive(Reflect, PoolLabel, FromTemplate, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component, PartialEq, Debug, Hash, Clone)]
pub struct MusicPool;

#[derive(Reflect, NodeLabel, FromTemplate, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component, PartialEq, Debug, Hash, Clone)]
pub struct MusicBus;

#[derive(Reflect, NodeLabel, FromTemplate, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component, PartialEq, Debug, Hash, Clone)]
pub struct MusicVolume;

#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct SoundtrackPlay {
    pub entity: Entity,
}

pub fn setup_player_bus(mut commands: Commands) {
    commands
        .spawn((MultibandCompressor::maximus_preset_a(), MusicBus))
        .chain_node((VolumeNode::default(), MusicVolume));
    commands
        .spawn((SamplerPool(MusicPool), PoolSize(12..=36), sample_effects![VolumeNode::default()]))
        .connect(MusicBus);
}

pub fn insert_player_state(
    mut commands: Commands,
    soundtracks: Res<Assets<Soundtrack>>,
    players: Query<(Entity, &SoundtrackPlayer, &mut SoundtrackState), Or<(Changed<SoundtrackPlayer>, AssetChanged<SoundtrackPlayer>)>>,
) {
    for (entity, player, mut state) in players {
        let Some(soundtrack) = soundtracks.get(&player.0) else { continue };
        for (&key, sample) in &soundtrack.entries {
            let sample_bundle = (
                SamplePlayer::new(sample.clone()).looping(),
                SamplerConfig {
                    num_declickers: 0,
                    ..default()
                },
                PlaybackSettings::default().preserve(),
                MusicPool,
            );

            match state.entries.entry(key) {
                Entry::Occupied(entry) => {
                    commands.entity(*entry.get()).insert(sample_bundle);
                }
                Entry::Vacant(entry) => {
                    entry.insert(commands.spawn((ChildOf(entity), sample_bundle)).id());
                }
            }
        }

        state.entries.retain(|key, &mut sample_entity| {
            if soundtrack.entries.contains_key(key) {
                true
            } else {
                commands.entity(sample_entity).try_despawn();
                false
            }
        });

        commands.trigger(SoundtrackPlay { entity });
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup_player_bus).add_systems(PostUpdate, insert_player_state);
}
