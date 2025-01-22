pub use big_space::prelude::{BigSpaceCommands, FloatingOrigin};

pub type GridPrecision = i32;

pub type BigSpacePlugin = big_space::prelude::BigSpacePlugin<GridPrecision>;
pub type Grid = big_space::prelude::Grid<GridPrecision>;
pub type Grids<'w, 's> = big_space::prelude::Grids<'w, 's, GridPrecision>;
pub type GridCell = big_space::prelude::GridCell<GridPrecision>;
pub type GridTransform = big_space::world_query::GridTransform<GridPrecision>;
pub type GridTransformReadOnly = big_space::world_query::GridTransformReadOnly<GridPrecision>;
pub type GridTransformOwned = big_space::world_query::GridTransformOwned<GridPrecision>;
pub type GridTransformItem<'w> = big_space::world_query::GridTransformItem<'w, GridPrecision>;
