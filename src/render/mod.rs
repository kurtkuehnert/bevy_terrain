pub mod compute_pipelines;
pub mod culling;
pub mod layouts;
pub mod render_pipeline;
pub mod shaders;
pub mod terrain_data;
pub mod terrain_view_data;

/// Configures the default terrain pipeline.
pub struct TerrainPipelineConfig {
    /// The number of terrain attachments.
    pub attachment_count: usize,
}

impl Default for TerrainPipelineConfig {
    fn default() -> Self {
        Self {
            attachment_count: 2,
        }
    }
}
