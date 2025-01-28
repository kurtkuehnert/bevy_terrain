use crate::prelude::{
    TerrainConfig, TerrainMaterial, TerrainSettings, TerrainViewComponents, TerrainViewConfig,
    TileAtlas, TileTree,
};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use big_space::floating_origins::BigSpace;

#[derive(Clone)]
pub(crate) struct TerrainToSpawn<M: Material + Clone> {
    config: Handle<TerrainConfig>,
    view_config: TerrainViewConfig,
    material: M,
    view: Entity,
}

#[derive(Resource)]
pub(crate) struct TerrainsToSpawn<M: Material>(pub(crate) Vec<TerrainToSpawn<M>>);

pub(crate) fn spawn_terrains<M: Material>(
    mut commands: Commands,
    mut terrains: ResMut<TerrainsToSpawn<M>>,
    asset_server: Res<AssetServer>,
) {
    terrains.0.retain(|terrain| {
        if asset_server.is_loaded(&terrain.config) {
            let terrain = terrain.clone();

            commands.queue(move |world: &mut World| {
                let TerrainToSpawn {
                    config,
                    view_config,
                    material,
                    view,
                } = terrain;

                let mut state = SystemState::<(
                    Commands,
                    Res<Assets<TerrainConfig>>,
                    Query<Entity, With<BigSpace>>,
                    ResMut<Assets<M>>,
                    ResMut<TerrainViewComponents<TileTree>>,
                    ResMut<Assets<ShaderStorageBuffer>>,
                    Res<TerrainSettings>,
                )>::new(world);

                let (
                    mut commands,
                    configs,
                    big_space,
                    mut materials,
                    mut tile_trees,
                    mut buffers,
                    settings,
                ) = state.get_mut(world);

                let config = configs.get(config.id()).unwrap().clone();

                let root = big_space.single();

                let terrain = commands
                    .spawn((
                        config.shape.transform(),
                        TileAtlas::new(&config, &mut buffers, &settings),
                        TerrainMaterial(materials.add(material)),
                    ))
                    .id();

                commands.entity(root).add_child(terrain);

                tile_trees.insert(
                    (terrain, view),
                    TileTree::new(
                        &config,
                        &view_config,
                        (terrain, view),
                        &mut commands,
                        &mut buffers,
                    ),
                );

                state.apply(world);
            });
            false
        } else {
            true
        }
    });
}

pub trait SpawnTerrainCommandsExt<M: Material> {
    // define a method that we will be able to call on `commands`
    fn spawn_terrain(
        &mut self,
        config: Handle<TerrainConfig>,
        view_config: TerrainViewConfig,
        material: M,
        view: Entity,
    );
}

impl<'w, 's, M: Material> SpawnTerrainCommandsExt<M> for Commands<'w, 's> {
    fn spawn_terrain(
        &mut self,
        config: Handle<TerrainConfig>,
        view_config: TerrainViewConfig,
        material: M,
        view: Entity,
    ) {
        self.queue(move |world: &mut World| {
            world
                .resource_mut::<TerrainsToSpawn<M>>()
                .0
                .push(TerrainToSpawn {
                    config,
                    view_config,
                    material,
                    view,
                });
        });
    }
}
