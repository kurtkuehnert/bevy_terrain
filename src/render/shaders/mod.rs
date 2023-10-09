use bevy::asset::load_internal_asset;
use bevy::{prelude::*, reflect::TypeUuid};

const TYPES_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 907665645684322571);
const BINDINGS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 570929401458920492);
const FUNCTIONS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 234313897973543254);
const DEBUG_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 513467378691355413);
const MINMAX_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 705341350987806053);
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

pub(crate) fn load_terrain_shaders(app: &mut App) {
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
