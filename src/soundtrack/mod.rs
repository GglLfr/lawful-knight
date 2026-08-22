use crate::prelude::*;

mod asset;
mod player;
pub use asset::*;
pub use player::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((asset::plugin, player::plugin));
}
