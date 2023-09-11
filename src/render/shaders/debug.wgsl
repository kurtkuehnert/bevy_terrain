#define_import_path bevy_terrain::debug
#import bevy_terrain::types TerrainConfig,TerrainViewConfig,Tile,TileList
#import bevy_terrain::functions calculate_morph, minmax
#import bevy_terrain::parameters Parameters

// view bindings
#import bevy_pbr::mesh_view_bindings view

//view bindings
@group(0) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(0) @binding(1)
var quadtree: texture_2d_array<u32>;
@group(0) @binding(2)
var<storage, read_write> final_tiles: TileList;
@group(0) @binding(3)
var<storage, read_write> temporary_tiles: TileList;
@group(0) @binding(4)
var<storage, read_write> parameters: Parameters;

// terrain bindings
@group(2) @binding(0)
var<uniform> config: TerrainConfig;
@group(2) @binding(1)
var atlas_sampler: sampler;
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var minmax_atlas: texture_2d_array<f32>;

fn lod_color(lod: u32) -> vec4<f32> {
    if (lod % 6u == 0u) {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (lod % 6u == 1u) {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 2u) {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 3u) {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 4u) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 5u) {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    return vec4<f32>(0.0);
}

fn show_tiles(tile: Tile, world_position: vec4<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    if ((tile.coords.x + tile.coords.y) % 2u == 0u) {
        color = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    }
    else {
        color = vec4<f32>(0.1, 0.1, 0.1, 1.0);
    }

    let lod = u32(ceil(log2(f32(tile.size))));
    color = mix(color, lod_color(lod), 0.5);

#ifdef MESH_MORPH
    let morph = calculate_morph(tile, world_position );
    color = color + vec4<f32>(1.0, 1.0, 1.0, 1.0) * morph;
#endif

    return vec4<f32>(color.xyz, 0.5);
}

fn show_minmax_error(tile: Tile, height: f32) -> vec4<f32> {
    let size = f32(tile.size) * view_config.tile_scale;
    let local_position = (vec2<f32>(tile.coords) + 0.5) * size;
    let lod = u32(ceil(log2(size))) + 1u;
    let minmax = minmax(local_position, size );

    var color = vec4<f32>(0.0,
                          clamp((minmax.y - height) / size / 2.0, 0.0, 1.0),
                          clamp((height - minmax.x) / size / 2.0, 0.0, 1.0),
                          0.5);

    let tolerance = 0.00001;

    if (height < minmax.x - tolerance || height > minmax.y + tolerance || lod >= config.lod_count) {
        color = vec4<f32>(1.0, 0.0, 0.0, 0.5);
    }

    return color;
}

fn show_lod(lod: u32, world_position: vec3<f32>) -> vec4<f32> {
    var color = lod_color(lod);

    for (var i = 0u; i < config.lod_count; i = i + 1u) {
        let viewer_distance = distance(view.world_position.xyz, world_position);
        let circle = f32(1u << i) * view_config.blend_distance;

        if (viewer_distance < circle && circle - f32(8 << i) < viewer_distance) {
            color = lod_color(i) * 10.0;
        }

#ifdef SHOW_NODES
        let node_size = node_size(i);
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
