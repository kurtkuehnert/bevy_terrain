use crate::{
    big_space::GridCell,
    math::{TerrainShape, TileCoordinate},
    plugin::TerrainSettings,
    render::terrain_bind_group::TerrainUniform,
    terrain::TerrainConfig,
    terrain_data::{
        attachment::{AttachmentConfig, AttachmentData, AttachmentFormat, AttachmentLabel},
        tile_loader::DefaultLoader,
        tile_tree::{TileTree, TileTreeEntry},
    },
    terrain_view::TerrainViewComponents,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{render_resource::*, storage::ShaderStorageBuffer, view::NoFrustumCulling},
    tasks::Task,
    utils::{HashMap, HashSet},
};
use itertools::assert_equal;
use std::{collections::VecDeque, ops::DerefMut, path::PathBuf};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ShaderType)]
pub struct AtlasTile {
    pub(crate) coordinate: TileCoordinate,
    #[size(16)]
    pub(crate) atlas_index: u32,
}

impl AtlasTile {
    pub fn new(tile_coordinate: TileCoordinate, atlas_index: u32) -> Self {
        Self {
            coordinate: tile_coordinate,
            atlas_index,
        }
    }
}

impl From<AtlasTileAttachment> for AtlasTile {
    fn from(tile: AtlasTileAttachment) -> Self {
        Self {
            coordinate: tile.coordinate,
            atlas_index: tile.atlas_index,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TileAttachment {
    pub(crate) coordinate: TileCoordinate,
    pub(crate) label: AttachmentLabel,
}

#[derive(Clone, Debug, Default)]
pub struct AtlasTileAttachment {
    pub(crate) coordinate: TileCoordinate,
    pub(crate) atlas_index: u32,
    pub(crate) label: AttachmentLabel,
}

#[derive(Clone)]
pub(crate) struct AtlasTileAttachmentWithData {
    pub(crate) coordinate: TileCoordinate,
    pub(crate) atlas_index: u32,
    pub(crate) label: AttachmentLabel,
    pub(crate) data: AttachmentData,
}

/// An attachment of a [`TileAtlas`].
pub struct AtlasAttachment {
    pub(crate) path: PathBuf,
    pub(crate) texture_size: u32,
    pub(crate) center_size: u32,
    pub(crate) border_size: u32,
    pub(crate) mip_level_count: u32,
    pub(crate) format: AttachmentFormat,
}

impl AtlasAttachment {
    fn new(config: &AttachmentConfig, path: &str) -> Self {
        let path = if path.starts_with("assets") {
            path[7..].to_string()
        } else {
            path.to_string()
        };
        // let path = format!("assets/{path}/data/{name}");

        Self {
            path: PathBuf::from(path),
            texture_size: config.texture_size,
            center_size: config.center_size(),
            border_size: config.border_size,
            mip_level_count: config.mip_level_count,
            format: config.format,
        }
    }
}

/// The current state of a tile of a [`TileAtlas`].
///
/// This indicates, whether the tile is loading or loaded and ready to be used.
#[derive(Clone, Copy, Debug)]
enum LoadingState {
    /// The tile is loading, but can not be used yet.
    Loading(u32),
    /// The tile is loaded and can be used.
    Loaded,
}

/// The internal representation of a present tile in a [`TileAtlas`].
struct TileState {
    coordinate: TileCoordinate,
    /// Indicates whether or not the tile is loading or loaded.
    state: LoadingState,
    /// The index of the tile inside the atlas.
    atlas_index: u32,
    /// The count of [`TileTrees`] that have requested this tile.
    requests: u32,
}

// Todo: rename to terrain?
// Todo: consider turning this into an asset

/// A sparse storage of all terrain attachments, which streams data in and out of memory
/// depending on the decisions of the corresponding [`TileTree`]s.
///
/// A tile is considered present and assigned an [`u32`] as soon as it is
/// requested by any tile_tree. Then the tile atlas will start loading all of its attachments
/// by storing the [`TileCoordinate`] (for one frame) in `load_events` for which
/// attachment-loading-systems can listen.
/// Tiles that are not being used by any tile_tree anymore are cached (LRU),
/// until new atlas indices are required.
///
/// The [`u32`] can be used for accessing the attached data in systems by the CPU
/// and in shaders by the GPU.
#[derive(Component)]
#[require(Transform, Visibility, NoFrustumCulling, DefaultLoader)]
#[cfg_attr(feature = "high_precision", require(GridCell))]
pub struct TileAtlas {
    pub(crate) attachments: HashMap<AttachmentLabel, AtlasAttachment>, // stores the attachment data
    tile_states: HashMap<TileCoordinate, TileState>,
    unused_indices: VecDeque<u32>,
    existing_tiles: HashSet<TileCoordinate>,
    pub(crate) uploading_tiles: Vec<AtlasTileAttachmentWithData>,
    pub(crate) downloading_tiles: Vec<Task<AtlasTileAttachmentWithData>>,
    pub(crate) to_load: Vec<TileAttachment>,

    pub(crate) lod_count: u32,
    pub(crate) min_height: f32,
    pub(crate) max_height: f32,
    pub(crate) height_scale: f32,
    pub(crate) shape: TerrainShape,

    pub(crate) terrain_buffer: Handle<ShaderStorageBuffer>,
}

impl TileAtlas {
    /// Creates a new tile_tree from a terrain config.
    pub fn new(
        config: &TerrainConfig,
        buffers: &mut Assets<ShaderStorageBuffer>,
        settings: &TerrainSettings,
    ) -> Self {
        let attachments = config
            .attachments
            .iter()
            .map(|(label, attachment)| {
                (
                    label.clone(),
                    AtlasAttachment::new(attachment, &config.path),
                )
            })
            .collect();

        let terrain_buffer = buffers.add(ShaderStorageBuffer::with_size(
            TerrainUniform::min_size().get() as usize,
            RenderAssetUsages::all(),
        ));

        Self {
            attachments,
            tile_states: default(),
            unused_indices: (0..settings.atlas_size).collect(),
            existing_tiles: HashSet::from_iter(config.tiles.clone()),
            to_load: default(),
            uploading_tiles: default(),
            downloading_tiles: default(),
            lod_count: config.lod_count,
            min_height: config.min_height,
            max_height: config.max_height,
            height_scale: 1.0,
            shape: config.shape,
            terrain_buffer,
        }
    }

    pub(crate) fn get_best_tile(&self, tile_coordinate: TileCoordinate) -> TileTreeEntry {
        let mut best_tile_coordinate = tile_coordinate;

        loop {
            if best_tile_coordinate == TileCoordinate::INVALID {
                // highest lod is not loaded
                return TileTreeEntry::default();
            }

            if let Some(tile) = self.tile_states.get(&best_tile_coordinate) {
                if matches!(tile.state, LoadingState::Loaded) {
                    // found best loaded tile
                    return TileTreeEntry {
                        atlas_index: tile.atlas_index,
                        atlas_lod: best_tile_coordinate.lod,
                    };
                }
            }

            best_tile_coordinate = best_tile_coordinate
                .parent()
                .unwrap_or(TileCoordinate::INVALID);
        }
    }

    pub(crate) fn tile_loaded(&mut self, tile: TileAttachment, data: AttachmentData) {
        if let Some(tile_state) = self.tile_states.get_mut(&tile.coordinate) {
            tile_state.state = match tile_state.state {
                LoadingState::Loading(1) => LoadingState::Loaded,
                LoadingState::Loading(n) => LoadingState::Loading(n - 1),
                LoadingState::Loaded => {
                    panic!("Loaded more attachments, than registered with the tile atlas.")
                }
            };

            self.uploading_tiles.push(AtlasTileAttachmentWithData {
                coordinate: tile.coordinate,
                atlas_index: tile_state.atlas_index,
                label: tile.label,
                data,
            });
        } else {
            dbg!("Tile is no longer loaded.");
        }
    }

    /// Updates the tile atlas according to all corresponding tile_trees.
    pub(crate) fn update(
        mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
        mut tile_atlases: Query<&mut TileAtlas>,
    ) {
        for (&(terrain, _view), tile_tree) in tile_trees.iter_mut() {
            let mut tile_atlas = tile_atlases.get_mut(terrain).unwrap();

            for tile_coordinate in tile_tree.released_tiles.drain(..) {
                tile_atlas.release_tile(tile_coordinate);
            }

            for tile_coordinate in tile_tree.requested_tiles.drain(..) {
                tile_atlas.request_tile(tile_coordinate);
            }

            let TileAtlas {
                tile_states,
                uploading_tiles,
                to_load,
                ..
            } = tile_atlas.deref_mut();

            // to_load.retain(|tile| {
            //     if let Some(tile) = tile_states.get(&tile.coordinate) {
            //         tile.requests > 0
            //     } else {
            //         dbg!("Tile is no longer needed.");
            //         false
            //     }
            // });
            //
            // uploading_tiles.retain(|tile| {
            //     if let Some(tile) = tile_states.get(&tile.coordinate) {
            //         tile.requests > 0
            //     } else {
            //         dbg!("Tile is no longer needed.");
            //         false
            //     }
            // });

            // dbg!(tile_states.len());
            // dbg!(tile_atlas.unused_tiles.len());
        }
    }

    pub fn update_terrain_buffer(
        mut tile_atlases: Query<(&mut TileAtlas, &GlobalTransform)>,
        mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    ) {
        for (tile_atlas, global_transform) in &mut tile_atlases {
            let terrain_buffer = buffers.get_mut(&tile_atlas.terrain_buffer).unwrap();
            terrain_buffer.set_data(TerrainUniform::new(&tile_atlas, global_transform));
        }
    }

    fn request_tile(&mut self, tile_coordinate: TileCoordinate) {
        if !self.existing_tiles.contains(&tile_coordinate) {
            return;
        }

        // check if the tile is already present else start loading it
        if let Some(tile) = self.tile_states.get_mut(&tile_coordinate) {
            if tile.requests == 0 {
                // the tile is now used again

                panic!("never happens!");

                // if !matches!(tile.state, LoadingState::Loaded) {
                //     panic!("Tile, that is not loaded is reused!");
                // }
                //
                // self.unused_tiles
                //     .retain(|unused_tile| tile.atlas_index != unused_tile.atlas_index);
            }

            tile.requests += 1;
        } else {
            let atlas_index = self
                .unused_indices
                .pop_front()
                .expect("Atlas out of indices");

            self.tile_states
                .retain(|_, tile| tile.atlas_index != atlas_index); // remove tile if it is still cached

            self.tile_states.insert(
                tile_coordinate,
                TileState {
                    coordinate: tile_coordinate,
                    requests: 1,
                    state: LoadingState::Loading(self.attachments.len() as u32),
                    atlas_index,
                },
            );

            for label in self.attachments.keys() {
                self.to_load.push(TileAttachment {
                    coordinate: tile_coordinate,
                    label: label.clone(),
                });
            }
        }
    }

    fn release_tile(&mut self, tile_coordinate: TileCoordinate) {
        if !self.existing_tiles.contains(&tile_coordinate) {
            return;
        }

        let tile = self.tile_states.get_mut(&tile_coordinate).unwrap();
        tile.requests -= 1;

        if tile.requests == 0 {
            self.unused_indices.push_back(tile.atlas_index);
            self.tile_states.remove(&tile_coordinate);

            // if !matches!(tile.state, LoadingState::Loaded) {
            //     dbg!("Cancel loading tile.");
            //     // the tile is not fully loaded
            //     // We would rather discard the current progress, instead of finish loading a tile we do not need anymore.
            //     // tile.state = LoadingState::Canceled;
            //     self.tile_states.remove(&tile_coordinate);
            // }
        }
    }
}
