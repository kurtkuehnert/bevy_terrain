use crate::{
    attachment_loader::{finish_loading_attachment_from_disk, start_loading_attachment_from_disk},
    formats::{tc::load_node_config, TDFPlugin},
    preprocess::BaseConfig,
    render::{
        compute_pipelines::{
            queue_terrain_compute_pipelines, TerrainComputeNode, TerrainComputePipelines,
        },
        culling_bind_group::{prepare_culling_bind_group, CullingBindGroup},
        shaders::load_terrain_shaders,
        terrain_bind_group::{initialize_terrain_bind_group, TerrainBindGroup},
        terrain_view_data::{
            extract_terrain_view_config, initialize_terrain_view_data, prepare_terrain_view_config,
            TerrainViewConfigUniform, TerrainViewData,
        },
    },
    terrain::{Terrain, TerrainComponents, TerrainConfig},
    terrain_data::{
        gpu_node_atlas::{
            extract_node_atlas, initialize_gpu_node_atlas, prepare_node_atlas, GpuNodeAtlas,
        },
        gpu_quadtree::{extract_quadtree, initialize_gpu_quadtree, prepare_quadtree, GpuQuadtree},
        node_atlas::update_node_atlas,
        quadtree::{adjust_quadtree, compute_quadtree_request, Quadtree},
        AttachmentConfig,
    },
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::{
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin, main_graph::node::CAMERA_DRIVER,
        render_graph::RenderGraph, render_resource::*, Render, RenderApp, RenderSet,
    },
};

#[derive(Clone, Resource)]
pub struct TerrainPluginConfig {
    pub base: BaseConfig,
    pub attachments: Vec<AttachmentConfig>,
}

impl TerrainPluginConfig {
    pub fn with_base_attachment(base: BaseConfig) -> Self {
        Self {
            base,
            attachments: vec![base.height_attachment()],
        }
    }

    pub fn add_attachment(mut self, attachment: AttachmentConfig) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn configure_terrain(
        &self,
        side_length: f32,
        lod_count: u32,
        min_height: f32,
        max_height: f32,
        node_atlas_size: u32,
        path: String,
    ) -> TerrainConfig {
        let leaf_node_size = (self.base.texture_size - 2 * self.base.border_size) as f32;
        let leaf_node_count = side_length / leaf_node_size;

        let attachments = self
            .attachments
            .clone()
            .into_iter()
            .map(AttachmentConfig::into)
            .collect();

        let nodes = load_node_config(&path);

        TerrainConfig {
            lod_count,
            min_height,
            max_height,
            leaf_node_count,
            node_atlas_size,
            path,
            attachments,
            nodes,
        }
    }
}

/// The plugin for the terrain renderer.
pub struct TerrainPlugin {
    pub config: TerrainPluginConfig,
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TDFPlugin,
            ExtractComponentPlugin::<Terrain>::default(),
            ExtractComponentPlugin::<TerrainView>::default(),
        ))
        .insert_resource(self.config.clone())
        .init_resource::<TerrainViewComponents<Quadtree>>()
        .init_resource::<TerrainViewComponents<TerrainViewConfig>>()
        .add_systems(
            Last,
            (
                finish_loading_attachment_from_disk.before(update_node_atlas),
                compute_quadtree_request.before(update_node_atlas),
                update_node_atlas,
                adjust_quadtree.after(update_node_atlas),
                start_loading_attachment_from_disk.after(update_node_atlas),
                // update_height_under_viewer.after(adjust_quadtree),
            ),
        );

        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainComponents<GpuNodeAtlas>>()
            .init_resource::<TerrainComponents<TerrainBindGroup>>()
            .init_resource::<TerrainViewComponents<GpuQuadtree>>()
            .init_resource::<TerrainViewComponents<TerrainViewData>>()
            .init_resource::<TerrainViewComponents<TerrainViewConfigUniform>>()
            .init_resource::<TerrainViewComponents<CullingBindGroup>>()
            .add_systems(
                ExtractSchedule,
                (
                    extract_terrain_view_config,
                    initialize_gpu_node_atlas,
                    initialize_gpu_quadtree,
                    initialize_terrain_bind_group.after(initialize_gpu_node_atlas),
                    initialize_terrain_view_data.after(initialize_gpu_quadtree),
                    extract_node_atlas.after(initialize_gpu_node_atlas),
                    extract_quadtree.after(initialize_gpu_quadtree),
                ),
            )
            .add_systems(
                Render,
                queue_terrain_compute_pipelines.in_set(RenderSet::Queue),
            )
            .add_systems(
                Render,
                (
                    prepare_quadtree,
                    prepare_node_atlas,
                    prepare_terrain_view_config,
                    prepare_culling_bind_group,
                )
                    .in_set(RenderSet::Prepare),
            );
    }

    fn finish(&self, app: &mut App) {
        load_terrain_shaders(app, &self.config);

        let render_app = app
            .sub_app_mut(RenderApp)
            .init_resource::<TerrainComputePipelines>()
            .init_resource::<SpecializedComputePipelines<TerrainComputePipelines>>();

        let compute_node = TerrainComputeNode::from_world(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("terrain_compute", compute_node);
        render_graph.add_node_edge("terrain_compute", CAMERA_DRIVER);
    }
}
