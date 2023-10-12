use crate::plugin::TerrainPluginConfig;
use bevy::{asset::load_internal_asset, prelude::*};

const TYPES_SHADER: Handle<Shader> = Handle::weak_from_u128(907665645684322571);
const ATTACHMENTS_SHADER: Handle<Shader> = Handle::weak_from_u128(455702068769385768);
const BINDINGS_SHADER: Handle<Shader> = Handle::weak_from_u128(570929401458920492);
const FUNCTIONS_SHADER: Handle<Shader> = Handle::weak_from_u128(234313897973543254);
const DEBUG_SHADER: Handle<Shader> = Handle::weak_from_u128(513467378691355413);
const MINMAX_SHADER: Handle<Shader> = Handle::weak_from_u128(705341350987806053);
const VERTEX_SHADER: Handle<Shader> = Handle::weak_from_u128(187371091254673438);
const FRAGMENT_SHADER: Handle<Shader> = Handle::weak_from_u128(312347731894135735);

pub(crate) const PREPARE_INDIRECT_SHADER: Handle<Shader> =
    Handle::weak_from_u128(242384313596767307);
pub(crate) const REFINE_TILES_SHADER: Handle<Shader> = Handle::weak_from_u128(938732132468373352);

pub(crate) const DEFAULT_SHADER: Handle<Shader> = Handle::weak_from_u128(556563744564564658);

pub(crate) fn load_terrain_shaders(app: &mut App, plugin_config: &TerrainPluginConfig) {
    let attachments_shader = generate_attachment_shader(plugin_config);

    let mut assets = app.world.resource_mut::<Assets<Shader>>();
    assets.insert(ATTACHMENTS_SHADER, attachments_shader);

    load_internal_asset!(app, TYPES_SHADER, "types.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, BINDINGS_SHADER, "bindings.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, FUNCTIONS_SHADER, "functions.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, DEBUG_SHADER, "debug.wgsl", Shader::from_wgsl);

    load_internal_asset!(app, MINMAX_SHADER, "render/minmax.wgsl", Shader::from_wgsl);
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
            include_str!("attachments.wgsl"),
            binding = binding,
            attachment_name_lower = attachment_name_lower,
            attachment_name_upper = attachment_name_upper,
            attachment_size = attachment_size,
            attachment_scale = attachment_scale,
            attachment_offset = attachment_offset
        ));
    }

    Shader::from_wgsl(
        source,
        std::path::Path::new(file!())
            .parent()
            .unwrap()
            .join("attachments.wgsl")
            .to_string_lossy(),
    )
}
