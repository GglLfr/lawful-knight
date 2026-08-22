use crate::prelude::*;

pub mod effects;

mod asset;
mod labels;
mod player;
pub use asset::*;
pub use labels::*;
pub use player::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((effects::plugin, asset::plugin, player::plugin));
}
