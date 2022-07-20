use bevy::{prelude::*, reflect::TypeUuid};

const CONFIG_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 907665645684322571);
const TILE_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 556563744564564658);
const PARAMETERS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 656456784512075658);
const ATLAS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 124345314345873273);
const TERRAIN_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 234313897973543254);
const DEBUG_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 513467378691355413);

pub(crate) const PREPARE_INDIRECT_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 242384313596767307);
pub(crate) const TESSELATION_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 938732132468373352);

pub(crate) fn add_shader(app: &mut App) {
    let mut assets = app.world.resource_mut::<Assets<_>>();

    assets.set_untracked(
        CONFIG_SHADER,
        Shader::from_wgsl(include_str!("config.wgsl")),
    );
    assets.set_untracked(TILE_SHADER, Shader::from_wgsl(include_str!("tile.wgsl")));
    assets.set_untracked(
        PARAMETERS_SHADER,
        Shader::from_wgsl(include_str!("parameters.wgsl")),
    );
    assets.set_untracked(ATLAS_SHADER, Shader::from_wgsl(include_str!("atlas.wgsl")));
    assets.set_untracked(
        TERRAIN_SHADER,
        Shader::from_wgsl(include_str!("terrain.wgsl")),
    );
    assets.set_untracked(DEBUG_SHADER, Shader::from_wgsl(include_str!("debug.wgsl")));
    assets.set_untracked(
        PREPARE_INDIRECT_SHADER,
        Shader::from_wgsl(include_str!("prepare_indirect.wgsl")),
    );
    assets.set_untracked(
        TESSELATION_SHADER,
        Shader::from_wgsl(include_str!("tessellation.wgsl")),
    );
}
