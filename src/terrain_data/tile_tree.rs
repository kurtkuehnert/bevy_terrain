use crate::{
    math::{Coordinate, TerrainModel, TileCoordinate},
    terrain_data::{sample_height, TileAtlas, INVALID_ATLAS_INDEX, INVALID_LOD},
    terrain_view::{TerrainViewComponents, TerrainViewConfig},
    util::inverse_mix,
};
use bevy::{
    math::{DVec2, DVec3},
    prelude::*,
};
use bytemuck::{Pod, Zeroable};
use itertools::iproduct;
use ndarray::Array4;
use std::iter;

/// The current state of a tile of a [`TileTree`].
///
/// This indicates, whether or not the tile should be loaded into the [`TileAtlas`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum RequestState {
    /// The tile should be loaded.
    Requested,
    /// The tile does not have to be loaded.
    Released,
}

/// The internal representation of a tile in a [`TileTree`].
struct TileState {
    /// The current tile coordinate at the tile_tree position.
    coordinate: TileCoordinate,
    /// Indicates, whether the tile is currently demanded or released.
    state: RequestState,
}

impl Default for TileState {
    fn default() -> Self {
        Self {
            coordinate: TileCoordinate::INVALID,
            state: RequestState::Released,
        }
    }
}

/// An entry of the [`TileTree`], used to access the best currently loaded tile
/// of the [`TileAtlas`] on the CPU.
///
/// These entries are synced each frame with their equivalent representations in the
/// [`GpuTileTree`](super::gpu_tile_tree::GpuTileTree) for access on the GPU.
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub(crate) struct TileTreeEntry {
    /// The atlas index of the best entry.
    pub(crate) atlas_index: u32,
    /// The atlas lod of the best entry.
    pub(crate) atlas_lod: u32,
}

impl Default for TileTreeEntry {
    fn default() -> Self {
        Self {
            atlas_index: INVALID_ATLAS_INDEX,
            atlas_lod: INVALID_LOD,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct TileLookup {
    pub(crate) atlas_index: u32,
    pub(crate) atlas_lod: u32,
    pub(crate) atlas_uv: Vec2,
}

impl TileLookup {
    pub(crate) const INVALID: Self = Self {
        atlas_index: INVALID_ATLAS_INDEX,
        atlas_lod: INVALID_LOD,
        atlas_uv: Vec2::ZERO,
    };
}

/// A quadtree-like view of a terrain, that requests and releases tiles from the [`TileAtlas`]
/// depending on the distance to the viewer.
///
/// It can be used to access the best currently loaded tile of the [`TileAtlas`].
/// Additionally its sends this data to the GPU via the
/// [`GpuTileTree`](super::gpu_tile_tree::GpuTileTree) so that it can be utilised
/// in shaders as well.
///
/// Each view (camera, shadow-casting light) that should consider the terrain has to
/// have an associated tile tree.
///
/// This tile tree is a "cube" with a size of (`tree_size`x`tree_size`x`lod_count`), where each layer
/// corresponds to a lod. These layers are wrapping (modulo `tree_size`), that means that
/// the tile tree is always centered under the viewer and only considers `tree_size` / 2 tiles
/// in each direction.
///
/// Each frame the tile tree determines the state of each tile via the
/// `compute_requests` methode.
/// After the [`TileAtlas`] has adjusted to these requests, the tile tree retrieves the best
/// currently loaded tiles from the tile atlas via the `adjust` methode, which can later be used to access the terrain data.
#[derive(Component)]
pub struct TileTree {
    /// The current cpu tile_tree data. This is synced each frame with the gpu tile_tree data.
    pub(crate) data: Array4<TileTreeEntry>,
    /// Tiles that are no longer required by this tile_tree.
    pub(crate) released_tiles: Vec<TileCoordinate>,
    /// Tiles that are requested to be loaded by this tile_tree.
    pub(crate) requested_tiles: Vec<TileCoordinate>,
    /// The internal tile states of the tile_tree.
    tiles: Array4<TileState>,
    /// The count of level of detail layers.
    lod_count: u32,
    /// The count of tiles in x and y direction per layer.
    pub(crate) tree_size: u32,
    pub(crate) geometry_tile_count: u32,
    pub(crate) refinement_count: u32,
    pub(crate) grid_size: u32,
    pub(crate) morph_range: f32,
    pub(crate) blend_range: f32,
    pub(crate) morph_distance: f64,
    pub(crate) blend_distance: f64,
    pub(crate) subdivision_distance: f64,
    pub(crate) load_distance: f64,
    pub(crate) precision_threshold_distance: f64,
    pub(crate) view_lod: u32,
    pub(crate) view_world_position: DVec3,
    pub(crate) approximate_height: f32,
    pub(crate) view_coordinates: [Coordinate; 6],
    #[cfg(feature = "high_precision")]
    pub(crate) surface_approximation: [crate::math::SurfaceApproximation; 6],
}

impl TileTree {
    /// Creates a new tile_tree from a terrain and a terrain view config.
    pub fn new(tile_atlas: &TileAtlas, view_config: &TerrainViewConfig) -> Self {
        let model = &tile_atlas.model;
        let scale = model.scale();

        Self {
            lod_count: tile_atlas.lod_count,
            tree_size: view_config.tree_size,
            geometry_tile_count: view_config.geometry_tile_count,
            refinement_count: view_config.refinement_count,
            grid_size: view_config.grid_size,
            morph_distance: view_config.morph_distance * scale,
            blend_distance: view_config.blend_distance * scale,
            load_distance: view_config.blend_distance * scale * (1.0 + view_config.load_tolerance),
            subdivision_distance: view_config.morph_distance
                * scale
                * (1.0 + view_config.subdivision_tolerance),
            morph_range: view_config.morph_range,
            blend_range: view_config.blend_range,
            precision_threshold_distance: view_config.precision_threshold_distance * scale,
            view_lod: view_config.view_lod,
            view_world_position: default(),
            approximate_height: (model.min_height + model.max_height) / 2.0,
            data: Array4::default((
                model.face_count() as usize,
                tile_atlas.lod_count as usize,
                view_config.tree_size as usize,
                view_config.tree_size as usize,
            )),
            tiles: Array4::default((
                model.face_count() as usize,
                tile_atlas.lod_count as usize,
                view_config.tree_size as usize,
                view_config.tree_size as usize,
            )),
            released_tiles: default(),
            requested_tiles: default(),
            view_coordinates: default(),
            #[cfg(feature = "high_precision")]
            surface_approximation: default(),
        }
    }

    fn compute_tree_xy(coordinate: Coordinate, tile_count: f64) -> DVec2 {
        // scale and clamp the coordinate to the tile tree bounds
        (coordinate.uv * tile_count).min(DVec2::splat(tile_count - 0.000001))
    }

    fn compute_origin(&self, view_coordinate: Coordinate, lod: u32) -> IVec2 {
        let tile_count = TileCoordinate::count(lod) as f64;
        let tree_xy = Self::compute_tree_xy(view_coordinate, tile_count);

        (tree_xy - 0.5 * self.tree_size as f64)
            .round()
            .clamp(
                DVec2::splat(0.0),
                DVec2::splat(tile_count - self.tree_size as f64),
            )
            .as_ivec2()
    }

    fn compute_tile_distance(
        &self,
        tile: TileCoordinate,
        view_coordinate: Coordinate,
        model: &TerrainModel,
    ) -> f64 {
        let tile_count = TileCoordinate::count(tile.lod) as f64;
        let view_tile_xy = Self::compute_tree_xy(view_coordinate, tile_count);
        let tile_offset = view_tile_xy.as_ivec2() - tile.xy;
        let mut offset = view_tile_xy % 1.0;

        if tile_offset.x < 0 {
            offset.x = 0.0;
        } else if tile_offset.x > 0 {
            offset.x = 1.0;
        }
        if tile_offset.y < 0 {
            offset.y = 0.0;
        } else if tile_offset.y > 0 {
            offset.y = 1.0;
        }

        let tile_world_position =
            Coordinate::new(tile.face, (tile.xy.as_dvec2() + offset) / tile_count)
                .world_position(model, self.approximate_height);

        tile_world_position.distance(self.view_world_position)
    }

    pub(crate) fn compute_blend(&self, sample_world_position: DVec3) -> (u32, f32) {
        let view_distance = self.view_world_position.distance(sample_world_position);
        let target_lod = (self.blend_distance / view_distance)
            .log2()
            .min(self.lod_count as f64 - 0.00001) as f32;
        let lod = target_lod as u32;

        let ratio = if lod == 0 {
            0.0
        } else {
            inverse_mix(lod as f32 + self.blend_range, lod as f32, target_lod)
        };

        (lod, ratio)
    }

    pub(crate) fn lookup_tile(
        &self,
        world_position: DVec3,
        tree_lod: u32,
        model: &TerrainModel,
    ) -> TileLookup {
        let coordinate = Coordinate::from_world_position(world_position, model);

        let tile_count = TileCoordinate::count(tree_lod) as f64;
        let tree_xy = Self::compute_tree_xy(coordinate, tile_count);

        let entry = self.data[[
            coordinate.face as usize,
            tree_lod as usize,
            tree_xy.x as usize % self.tree_size as usize,
            tree_xy.y as usize % self.tree_size as usize,
        ]];

        if entry.atlas_lod == INVALID_LOD {
            return TileLookup::INVALID;
        }

        TileLookup {
            atlas_index: entry.atlas_index,
            atlas_lod: entry.atlas_lod,
            atlas_uv: ((tree_xy / (1 << (tree_lod - entry.atlas_lod)) as f64) % 1.0).as_vec2(),
        }
    }

    fn update(&mut self, view_position: DVec3, tile_atlas: &TileAtlas) {
        let model = &tile_atlas.model;
        self.view_world_position = view_position;

        let view_coordinate = Coordinate::from_world_position(self.view_world_position, model);

        for face in 0..model.face_count() {
            let view_coordinate = view_coordinate.project_to_face(face);
            self.view_coordinates[face as usize] = view_coordinate;

            for lod in 0..tile_atlas.lod_count {
                let origin = self.compute_origin(view_coordinate, lod);

                for (x, y) in iproduct!(0..self.tree_size, 0..self.tree_size) {
                    let tile_coordinate = TileCoordinate {
                        face,
                        lod,
                        xy: origin + IVec2::new(x as i32, y as i32),
                    };

                    let tile_distance =
                        self.compute_tile_distance(tile_coordinate, view_coordinate, model);
                    let load_distance =
                        self.load_distance / TileCoordinate::count(tile_coordinate.lod) as f64;

                    let state = if lod == 0 || tile_distance < load_distance {
                        RequestState::Requested
                    } else {
                        RequestState::Released
                    };

                    let tile = &mut self.tiles[[
                        face as usize,
                        lod as usize,
                        (tile_coordinate.xy.x as usize % self.tree_size as usize),
                        (tile_coordinate.xy.y as usize % self.tree_size as usize),
                    ]];

                    // check if tile_tree slot refers to a new tile
                    if tile_coordinate != tile.coordinate {
                        // release old tile
                        if tile.state == RequestState::Requested {
                            tile.state = RequestState::Released;
                            self.released_tiles.push(tile.coordinate);
                        }

                        tile.coordinate = tile_coordinate;
                    }

                    // request or release tile based on its distance to the view
                    match (tile.state, state) {
                        (RequestState::Released, RequestState::Requested) => {
                            tile.state = RequestState::Requested;
                            self.requested_tiles.push(tile.coordinate);
                        }
                        (RequestState::Requested, RequestState::Released) => {
                            tile.state = RequestState::Released;
                            self.released_tiles.push(tile.coordinate);
                        }
                        (_, _) => {}
                    }
                }
            }
        }
    }

    /// Traverses all tile_trees and updates the tile states,
    /// while selecting newly requested and released tiles.
    pub(crate) fn compute_requests(
        mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
        tile_atlases: Query<&TileAtlas>,
        #[cfg(feature = "high_precision")] frames: crate::big_space::ReferenceFrames,
        #[cfg(feature = "high_precision")] view_transforms: Query<
            crate::big_space::GridTransformReadOnly,
        >,
        #[cfg(not(feature = "high_precision"))] view_transforms: Query<&Transform>,
    ) {
        for (&(terrain, view), tile_tree) in tile_trees.iter_mut() {
            let tile_atlas = tile_atlases.get(terrain).unwrap();
            let view_transform = view_transforms.get(view).unwrap();

            #[cfg(feature = "high_precision")]
            let frame = frames.parent_frame(terrain).unwrap();
            #[cfg(feature = "high_precision")]
            let view_position = view_transform.position_double(frame);
            #[cfg(not(feature = "high_precision"))]
            let view_position = view_transform.translation.as_dvec3();

            tile_tree.update(view_position, tile_atlas);
        }
    }

    /// Adjusts all tile_trees to their corresponding tile atlas
    /// by updating the entries with the best available tiles.
    pub(crate) fn adjust_to_tile_atlas(
        mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
        tile_atlases: Query<&TileAtlas>,
    ) {
        for (&(terrain, _view), tile_tree) in tile_trees.iter_mut() {
            let tile_atlas = tile_atlases.get(terrain).unwrap();

            for (tile, entry) in iter::zip(&tile_tree.tiles, &mut tile_tree.data) {
                *entry = tile_atlas.get_best_tile(tile.coordinate);
            }
        }
    }

    pub(crate) fn approximate_height(
        mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
        tile_atlases: Query<&TileAtlas>,
    ) {
        for (&(terrain, _view), tile_tree) in tile_trees.iter_mut() {
            let tile_atlas = tile_atlases.get(terrain).unwrap();

            let height = sample_height(tile_tree, tile_atlas, tile_tree.view_world_position);

            if height != 0.0 {
                dbg!(height);
                tile_tree.approximate_height = height;
            }
        }
    }

    #[cfg(feature = "high_precision")]
    pub fn generate_surface_approximation(
        mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
        tile_atlases: Query<&TileAtlas>,
    ) {
        for (&(terrain, _view), tile_tree) in tile_trees.iter_mut() {
            let tile_atlas = tile_atlases.get(terrain).unwrap();

            tile_tree.surface_approximation = tile_tree.view_coordinates.map(|view_coordinate| {
                crate::math::SurfaceApproximation::compute(
                    view_coordinate,
                    tile_tree.view_world_position,
                    &tile_atlas.model,
                )
            });
        }
    }
}
