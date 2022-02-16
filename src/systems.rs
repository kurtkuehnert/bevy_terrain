use crate::node_atlas::NodeAtlas;
use crate::quadtree::{NodeData, Nodes, Quadtree, Viewer};
use crate::{
    AssetEvent, AssetServer, Camera, EventReader, GlobalTransform, Image, Query, Res, ViewDistance,
    With,
};
use bevy::asset::Assets;
use bevy::math::Vec3Swizzles;
use bevy::prelude::ResMut;
use bevy::render::render_resource::TextureUsages;
use std::mem;

/// Traverses all quadtrees and generates a new tree update.
pub fn traverse_quadtree(
    viewer_query: Query<(&GlobalTransform, &ViewDistance), With<Camera>>,
    mut terrain_query: Query<(&GlobalTransform, &mut Quadtree)>,
) {
    for (terrain_transform, mut quadtree) in terrain_query.iter_mut() {
        for (camera_transform, view_distance) in viewer_query.iter() {
            let viewer = Viewer {
                position: (camera_transform.translation - terrain_transform.translation).xz(),
                view_distance: view_distance.view_distance,
            };

            quadtree.traverse(viewer);
        }
    }
}

/// Updates the nodes and the node atlas according to the corresponding tree update
/// and the load statuses.
pub fn update_nodes(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut Quadtree, &mut Nodes, &mut NodeAtlas)>,
) {
    for (mut quadtree, mut nodes, mut node_atlas) in terrain_query.iter_mut() {
        let Nodes {
            ref mut handle_mapping,
            ref mut load_statuses,
            ref mut loading_nodes,
            ref mut inactive_nodes,
            ref mut active_nodes,
        } = nodes.as_mut();

        // clear the previously activated nodes
        quadtree.activated_nodes.clear();

        let mut nodes_to_activate: Vec<NodeData> = Vec::new();

        // load required nodes from cache or disk
        for id in mem::take(&mut quadtree.nodes_to_activate) {
            if let Some(node) = inactive_nodes.pop(&id) {
                // queue cached node for activation
                nodes_to_activate.push(node);
            } else {
                // load node before activation
                loading_nodes.insert(
                    id,
                    NodeData::load(id, &asset_server, load_statuses, handle_mapping),
                );
            };
        }

        // queue all nodes, that have finished loading, for activation
        load_statuses.retain(|&id, status| {
            if status.finished {
                nodes_to_activate.push(loading_nodes.remove(&id).unwrap());
            }

            !status.finished
        });

        // deactivate all no longer required nodes
        for id in mem::take(&mut quadtree.nodes_to_deactivate) {
            let mut node = active_nodes.remove(&id).unwrap();
            node_atlas.deactivate_node(&mut node);
            inactive_nodes.put(id, node);
        }

        // activate as all nodes ready for activation
        for mut node in nodes_to_activate {
            node_atlas.activate_node(&mut node);
            quadtree.activated_nodes.insert(node.id);
            active_nodes.insert(node.id, node);
        }
    }
}

/// Updates the load status of a node for all of it newly loaded assets.
pub fn update_load_status(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut terrain_query: Query<&mut Nodes>,
) {
    for event in asset_events.iter() {
        if let AssetEvent::Created { handle } = event {
            for mut nodes in terrain_query.iter_mut() {
                if let Some(id) = nodes.handle_mapping.remove(&handle.id) {
                    let image = images.get_mut(handle).unwrap();

                    image.texture_descriptor.usage = TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST
                        | TextureUsages::TEXTURE_BINDING;
                    let status = nodes.load_statuses.get_mut(&id).unwrap();
                    status.finished = true;
                    break;
                }
            }
        }
    }
}
