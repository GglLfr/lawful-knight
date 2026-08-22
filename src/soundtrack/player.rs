use bevy::{asset::AsAssetId, ecs::intern::Interned};

use crate::{
    prelude::*,
    soundtrack::{Soundtrack, SoundtrackLabel},
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

pub fn insert_player_state(
    mut commands: Commands,
    soundtracks: Res<Assets<Soundtrack>>,
    players: Query<(Entity, &SoundtrackPlayer, &mut SoundtrackState), Or<(Changed<SoundtrackPlayer>, AssetChanged<SoundtrackPlayer>)>>,
) {
    // TODO relationship between master and nodes
    for (.., player, mut state) in players {
        for (.., e) in state.entries.drain() {
            commands.entity(e).try_despawn();
        }

        let Some(soundtrack) = soundtracks.get(&player.0) else { continue };
        for sample in soundtrack.entries.values() {
            commands.spawn(SamplePlayer::new(sample.clone()).looping());
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, insert_player_state);
}
