use crate::prelude::*;

mod compressor;
pub use compressor::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(compressor::plugin);
}
