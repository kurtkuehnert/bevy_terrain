use bevy::{asset::load_internal_asset, prelude::*, reflect::TypeUuid};

const TYPES_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 907665645684322571);
const PARAMETERS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 656456784512075658);
const NODE_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 124345314345873273);
const FUNCTIONS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 234313897973543254);
const DEBUG_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 513467378691355413);
const MINMAX_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 705341350987806053);
pub(crate) const VERTEX_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 187371091254673438);
pub(crate) const FRAGMENT_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 312347731894135735);
const UNIFORMS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 336468722148333179);

pub(crate) const PREPARE_INDIRECT_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 242384313596767307);
pub(crate) const REFINE_TILES_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 938732132468373352);

pub(crate) const DEFAULT_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 556563744564564658);

pub(crate) fn add_shader(app: &mut App) {
    // let mut assets = app.world.resource_mut::<Assets<_>>();
    // let mut shaders = app.world.resource_mut::<Assets<Shader>>();

    load_internal_asset!(app, TYPES_SHADER, "types.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, UNIFORMS_SHADER, "render/uniforms.wgsl", Shader::from_wgsl);

    load_internal_asset!(
        app,
        PARAMETERS_SHADER,
        "compute/parameters.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(app, NODE_SHADER, "node.wgsl", Shader::from_wgsl);
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
