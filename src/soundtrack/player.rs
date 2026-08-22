use crate::{
    prelude::*,
    soundtrack::{Soundtrack, SoundtrackLabel, effects::MultibandCompressor},
};

#[derive(Reflect, Component, Default, Debug, Clone)]
#[reflect(Component, Debug, Clone)]
#[require(SoundtrackState)]
pub struct SoundtrackPlayer(pub Handle<Soundtrack>);
impl AsAssetId for SoundtrackPlayer {
    type Asset = Soundtrack;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.0.id()
    }
}

#[derive(TypePath, Component, Default, Debug, Clone)]
pub struct SoundtrackState {
    pub entries: HashMap<Interned<dyn SoundtrackLabel>, Entity>,
}

#[derive(Reflect, PoolLabel, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component, PartialEq, Debug, Hash, Clone)]
pub struct MusicPool;

#[derive(Reflect, NodeLabel, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component, PartialEq, Debug, Hash, Clone)]
pub struct MusicBus;

#[derive(Reflect, NodeLabel, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component, PartialEq, Debug, Hash, Clone)]
pub struct MusicVolume;

pub fn setup_player_bus(mut commands: Commands) {
    commands
        .spawn((MultibandCompressor::default(), MusicBus))
        .chain_node((VolumeNode::default(), MusicVolume));
    commands.spawn(SamplerPool(MusicPool)).connect(MusicBus);
}

pub fn insert_player_state(
    mut commands: Commands,
    soundtracks: Res<Assets<Soundtrack>>,
    players: Query<(Entity, &SoundtrackPlayer, &mut SoundtrackState), Or<(Changed<SoundtrackPlayer>, AssetChanged<SoundtrackPlayer>)>>,
) {
    for (e, player, mut state) in players {
        for (.., e) in state.entries.drain() {
            commands.entity(e).try_despawn();
        }

        let Some(soundtrack) = soundtracks.get(&player.0) else { continue };
        for sample in soundtrack.entries.values() {
            commands.spawn((ChildOf(e), SamplePlayer::new(sample.clone()).looping(), MusicPool));
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup_player_bus).add_systems(PostUpdate, insert_player_state);
}
