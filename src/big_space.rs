pub use big_space::FloatingOrigin;

pub type GridPrecision = i32;

pub type FloatingOriginPlugin = big_space::FloatingOriginPlugin<GridPrecision>;
pub type RootReferenceFrame = big_space::reference_frame::RootReferenceFrame<GridPrecision>;
pub type GridCell = big_space::GridCell<GridPrecision>;
pub type GridTransform = big_space::world_query::GridTransform<GridPrecision>;
pub type GridTransformReadOnly = big_space::world_query::GridTransformReadOnly<GridPrecision>;
