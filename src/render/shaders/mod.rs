use bevy::{prelude::*, reflect::TypeUuid};

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

pub(crate) fn add_shader(app: &mut App) {
    let mut assets = app.world.resource_mut::<Assets<_>>();

    assets.set_untracked(TYPES_SHADER, Shader::from_wgsl(include_str!("types.wgsl")));

    assets.set_untracked(
        PARAMETERS_SHADER,
        Shader::from_wgsl(include_str!("compute/parameters.wgsl")),
    );
    assets.set_untracked(NODE_SHADER, Shader::from_wgsl(include_str!("node.wgsl")));
    assets.set_untracked(
        FUNCTIONS_SHADER,
        Shader::from_wgsl(include_str!("functions.wgsl")),
    );
    assets.set_untracked(DEBUG_SHADER, Shader::from_wgsl(include_str!("debug.wgsl")));

    assets.set_untracked(
        MINMAX_SHADER,
        Shader::from_wgsl(include_str!("render/minmax.wgsl")),
    );
    assets.set_untracked(
        VERTEX_SHADER,
        Shader::from_wgsl(include_str!("render/vertex.wgsl")),
    );
    assets.set_untracked(
        FRAGMENT_SHADER,
        Shader::from_wgsl(include_str!("render/fragment.wgsl")),
    );
    assets.set_untracked(
        DEFAULT_SHADER,
        Shader::from_wgsl(include_str!("render/default.wgsl")),
    );

    assets.set_untracked(
        PREPARE_INDIRECT_SHADER,
        Shader::from_wgsl(include_str!("compute/prepare_indirect.wgsl")),
    );
    assets.set_untracked(
        REFINE_TILES_SHADER,
        Shader::from_wgsl(include_str!("compute/refine_tiles.wgsl")),
    );
}
