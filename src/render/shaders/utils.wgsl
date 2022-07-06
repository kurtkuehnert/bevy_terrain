#define_import_path bevy_terrain::utils

struct VertexInput {
    [[builtin(instance_index)]] instance: u32;
    [[builtin(vertex_index)]] index: u32;
};

struct VertexOutput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] local_position: vec2<f32>;
    [[location(1)]] world_position: vec4<f32>;
    [[location(2)]] color: vec4<f32>;
};

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] local_position: vec2<f32>;
    [[location(1)]] world_position: vec4<f32>;
    [[location(2)]] color: vec4<f32>;
};

fn node_size(lod: u32) -> f32 {
    return f32(config.chunk_size * (1u << lod));
}

fn calculate_morph(local_position: vec2<f32>, patch: Patch) -> f32 {
    let world_position = vec3<f32>(local_position.x, view_config.height_under_viewer, local_position.y);
    let viewer_distance = distance(world_position, view.world_position.xyz);
    let morph_distance = f32(patch.size) * view_config.view_distance;

    return clamp(1.0 - (1.0 - viewer_distance / morph_distance) / morph_blend, 0.0, 1.0);
}

struct Blend {
    ratio: f32;
    log_distance: f32;
};

fn calculate_blend(world_position: vec3<f32>, blend_range: f32) -> Blend {
    let viewer_distance = distance(world_position, view.world_position.xyz);
    let log_distance = log2(2.0 * viewer_distance / view_config.view_distance);
    let ratio = (1.0 - log_distance % 1.0) / blend_range;

    return Blend(ratio, log_distance);
}



fn vertex_output(local_position: vec2<f32>, height: f32) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(local_position.x, height, local_position.y, 1.0);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = vec2<f32>(local_position);
    output.world_position = world_position;
    output.color = vec4<f32>(0.0);

    return output;
}

fn calculate_normal(uv: vec2<f32>, atlas_index: i32, lod: u32) -> vec3<f32> {
    let left  = textureSampleLevel(height_atlas, filter_sampler, uv, atlas_index, 0.0, vec2<i32>(-1,  0)).x;
    let up    = textureSampleLevel(height_atlas, filter_sampler, uv, atlas_index, 0.0, vec2<i32>( 0, -1)).x;
    let right = textureSampleLevel(height_atlas, filter_sampler, uv, atlas_index, 0.0, vec2<i32>( 1,  0)).x;
    let down  = textureSampleLevel(height_atlas, filter_sampler, uv, atlas_index, 0.0, vec2<i32>( 0,  1)).x;

    return normalize(vec3<f32>(right - left, f32(2u << lod) / config.height, down - up));
}



struct AtlasLookup {
    lod: u32;
    atlas_index: i32;
    atlas_coords: vec2<f32>;
};

fn atlas_lookup(log_distance: f32, local_position: vec2<f32>) -> AtlasLookup {
    let lod = clamp(u32(log_distance), 0u, config.lod_count - 1u);

#ifndef CIRCULAR_LOD
    for (var lod = 0u; lod < config.lod_count; lod = lod + 1u) {
        let coordinate = local_position / node_size(lod);
        let grid_coordinate = floor(view.world_position.xz / node_size(lod) + 0.5 - f32(view_config.node_count >> 1u));

        let grid = step(grid_coordinate, coordinate) * (1.0 - step(grid_coordinate + f32(view_config.node_count), coordinate));

        if (grid.x * grid.y == 1.0) {
            break;
        }
    }
#endif

    let map_coords = vec2<i32>((local_position / node_size(lod)) % f32(view_config.node_count));
    let lookup = textureLoad(quadtree, map_coords, i32(lod), 0);

    let atlas_lod = lookup.z;
    let atlas_index =  i32((lookup.x << 8u) + lookup.y);
    let atlas_coords = (local_position / node_size(atlas_lod)) % 1.0;

    return AtlasLookup(atlas_lod, atlas_index, atlas_coords);
}
