#define_import_path bevy_terrain::debug
#import bevy_terrain::types Tile
#import bevy_terrain::functions calculate_morph,minmax
#import bevy_terrain::uniforms view_config


fn lod_color(lod: u32) -> vec4<f32> {
    if lod % 6u == 0u {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if lod % 6u == 1u {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if lod % 6u == 2u {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if lod % 6u == 3u {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    if lod % 6u == 4u {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if lod % 6u == 5u {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    return vec4<f32>(0.0);
}

fn show_tiles(tile: Tile, world_position: vec4<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    if (tile.coords.x + tile.coords.y) % 2u == 0u {
        color = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    } else {
        color = vec4<f32>(0.1, 0.1, 0.1, 1.0);
    }

    let lod = u32(ceil(log2(f32(tile.size))));
    color = mix(color, lod_color(lod), 0.5);

#ifdef MESH_MORPH
    let morph = calculate_morph(tile, world_position);
    color = color + vec4<f32>(1.0, 1.0, 1.0, 1.0) * morph;
#endif

    return vec4<f32>(color.xyz, 0.5);
}

// struct TerrainViewConfig {
//     approximate_height: f32,
//     node_count: u32,

//     tile_count: u32,
//     refinement_count: u32,
//     tile_scale: f32,
//     grid_size: f32,
//     vertices_per_row: u32,
//     vertices_per_tile: u32,
//     morph_distance: f32,
//     blend_distance: f32,
//     morph_range: f32,
//     blend_range: f32,
// }

fn show_minmax_error(tile: Tile, height: f32, lod_count: u32, minmax_atlas: texture_2d_array<f32>, atlas_sampler: sampler, minmax_scale: f32, minmax_offset: f32, quadtree: texture_2d_array<u32>, leaf_node_size: u32) -> vec4<f32> {
    let size = f32(tile.size) * view_config.tile_scale;
    let local_position = (vec2<f32>(tile.coords) + 0.5) * size;
    let lod = u32(ceil(log2(size))) + 1u;
    // minmax_atlas: texture_2d_array<f32>, atlas_sampler: sampler, local_position: vec2<f32>, size: f32, lod_count: u32, height: f32, minmax_scale: f32, minmax_offset: f32, node_count: u32, quadtree: texture_2d_array<u32>, leaf_node_size: u32
    // minmax_atlas: texture_2d_array<f32>, atlas_sampler: sampler, local_position: vec2<f32>, size: f32, lod_count: u32, height: f32, minmax_scale: f32, minmax_offset: f32, quadtree: texture_2d_array<u32>, leaf_node_size: u32
    let minmax = minmax(minmax_atlas, atlas_sampler, local_position, size, lod_count, height, minmax_scale, minmax_offset, view_config.node_count, quadtree, leaf_node_size);

    var color = vec4<f32>(0.0, clamp((minmax.y - height) / size / 2.0, 0.0, 1.0), clamp((height - minmax.x) / size / 2.0, 0.0, 1.0), 0.5);

    let tolerance = 0.00001;

    if height < minmax.x - tolerance || height > minmax.y + tolerance || lod >= lod_count {
        color = vec4<f32>(1.0, 0.0, 0.0, 0.5);
    }

    return color;
}

fn show_lod(lod: u32, world_position: vec3<f32>, view_world_position: vec4<f32>, blend_distance: f32, lod_count: u32, leaf_node_size: u32) -> vec4<f32> {
    var color = lod_color(lod);

    for (var i = 0u; i < lod_count; i = i + 1u) {
        let viewer_distance = distance(view_world_position.xyz, world_position);
        let circle = f32(1u << i) * blend_distance;

        if viewer_distance < circle && circle - f32(8 << i) < viewer_distance {
            color = lod_color(i) * 10.0;
        }

#ifdef SHOW_NODES
        let node_size = node_size(i, leaf_node_size);
        let grid_position = floor(view.world_position.xz / node_size + 0.5 - f32(view_config.node_count >> 1u)) * node_size;
        let grid_size = node_size * f32(view_config.node_count);
        let thickness = f32(8u << i);

        let grid_outer = step(grid_position, world_position.xz) * step(world_position.xz, grid_position + grid_size);
        let grid_inner = step(grid_position + thickness, world_position.xz) * step(world_position.xz, grid_position + grid_size - thickness);
        let outline = grid_outer.x * grid_outer.y - grid_inner.x * grid_inner.y;

        color = mix(color, lod_color(i) * 10.0, outline);
#endif
    }

    return color;
}
