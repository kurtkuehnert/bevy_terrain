use crate::plugin::TerrainPluginConfig;
use bevy::{asset::load_internal_asset, prelude::*, reflect::TypeUuid};

const TYPES_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 907665645684322571);
const ATTACHMENTS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 455702068769385768);
const BINDINGS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 570929401458920492);
const FUNCTIONS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 234313897973543254);
const DEBUG_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 513467378691355413);
const VERTEX_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 187371091254673438);
const FRAGMENT_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 312347731894135735);

pub(crate) const PREPARE_INDIRECT_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 242384313596767307);
pub(crate) const REFINE_TILES_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 938732132468373352);

pub(crate) const DEFAULT_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 556563744564564658);

pub(crate) fn load_terrain_shaders(app: &mut App, plugin_config: &TerrainPluginConfig) {
    let attachments_shader = generate_attachment_shader(plugin_config);

    let mut assets = app.world.resource_mut::<Assets<Shader>>();
    assets.set_untracked(ATTACHMENTS_SHADER, attachments_shader);

    load_internal_asset!(app, TYPES_SHADER, "types.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, BINDINGS_SHADER, "bindings.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, FUNCTIONS_SHADER, "functions.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, DEBUG_SHADER, "debug.wgsl", Shader::from_wgsl);

    load_internal_asset!(app, VERTEX_SHADER, "render/vertex.wgsl", Shader::from_wgsl);
    load_internal_asset!(
        app,
        FRAGMENT_SHADER,
        "render/fragment.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(
        app,
        DEFAULT_SHADER,
        "render/default.wgsl",
        Shader::from_wgsl
    );

    load_internal_asset!(
        app,
        PREPARE_INDIRECT_SHADER,
        "compute/prepare_indirect.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(
        app,
        REFINE_TILES_SHADER,
        "compute/refine_tiles.wgsl",
        Shader::from_wgsl
    );
}

pub(crate) fn generate_attachment_shader(plugin_config: &TerrainPluginConfig) -> Shader {
    let mut source = String::from("#define_import_path bevy_terrain::attachments\r\n");

    for (i, attachment) in plugin_config.attachments.iter().enumerate() {
        let binding = 3 + i;
        let attachment_name_lower = attachment.name.to_lowercase();
        let attachment_name_upper = attachment.name.to_uppercase();

        let attachment_size = attachment.texture_size as f32;
        let attachment_scale = attachment.center_size as f32 / attachment.texture_size as f32;
        let attachment_offset = attachment.border_size as f32 / attachment.texture_size as f32;

        source.push_str(&format!(
            "
                const {attachment_name_upper}_SIZE  : f32 = {attachment_size:.10};
                const {attachment_name_upper}_SCALE : f32 = {attachment_scale:.10};
                const {attachment_name_upper}_OFFSET: f32 = {attachment_offset:.10};

                @group(2) @binding({binding})
                var {attachment_name_lower}_atlas: texture_2d_array<f32>;

            "
        ));
    }

    source.push_str(include_str!("attachments.wgsl"));

    Shader::from_wgsl(
        source,
        std::path::Path::new(file!())
            .parent()
            .unwrap()
            .join("attachments.wgsl")
            .to_string_lossy(),
    )
}
