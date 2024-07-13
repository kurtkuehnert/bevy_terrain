pub use big_space::{BigSpaceCommands, FloatingOrigin};

pub type GridPrecision = i32;

pub type BigSpacePlugin = big_space::BigSpacePlugin<GridPrecision>;
pub type ReferenceFrame = big_space::reference_frame::ReferenceFrame<GridPrecision>;
pub type ReferenceFrames<'w, 's> =
    big_space::reference_frame::local_origin::ReferenceFrames<'w, 's, GridPrecision>;
pub type GridCell = big_space::GridCell<GridPrecision>;
pub type GridTransform = big_space::world_query::GridTransform<GridPrecision>;
pub type GridTransformReadOnly = big_space::world_query::GridTransformReadOnly<GridPrecision>;
pub type GridTransformOwned = big_space::world_query::GridTransformOwned<GridPrecision>;
pub type GridTransformItem<'w> = big_space::world_query::GridTransformItem<'w, GridPrecision>;
