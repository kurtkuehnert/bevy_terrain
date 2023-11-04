#define_import_path bevy_terrain::vertex

#import bevy_terrain::types VertexInput, VertexOutput, NodeLookup
#import bevy_terrain::bindings config, view_config, tiles, atlas_sampler
#import bevy_terrain::functions vertex_local_position, approximate_world_position, blend, lookup_node
#import bevy_terrain::debug show_tiles
#import bevy_terrain::attachments height_atlas, HEIGHT_SCALE, HEIGHT_OFFSET
#import bevy_pbr::mesh_view_bindings view

fn terrain_world_position(height: f32, local_position: vec3<f32>) -> vec4<f32> {
    let height = config.height * height;

    let local_position = local_position + vec3<f32>(0.0, height, 0.0);

    return vec4<f32>(local_position, 1.0);
}

fn vertex_fn(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];

    let local_position = vertex_local_position(tile, grid_index);
    var world_position = approximate_world_position(local_position);

    var lod = blend(world_position).lod;
    let lookup = lookup_node(world_position, lod);
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let atlas_height = textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x;
    world_position = terrain_world_position(atlas_height, local_position);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = local_position;
    output.world_position = world_position;
    output.debug_color = show_tiles(tile, world_position);

    return output;
}
