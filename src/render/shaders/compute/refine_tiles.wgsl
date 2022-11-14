#import bevy_terrain::types
#import bevy_terrain::parameters

struct TerrainConfig {
    lod_count: u32,
    height: f32,
    leaf_node_size: u32,
    terrain_size: u32,

    height_size: f32,
    minmax_size: f32,
    _empty: u32,
    _empty: u32,
    height_scale: f32,
    minmax_scale: f32,
    _empty: u32,
    _empty: u32,
    height_offset: f32,
    minmax_offset: f32,
    _empty: u32,
    _empty: u32,
}

struct CullingData {
    world_position: vec4<f32>,
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    planes: array<vec4<f32>, 5>,
}

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

@group(1) @binding(0)
var<uniform> view: CullingData;

 // terrain bindings
@group(2) @binding(0)
var<uniform> config: TerrainConfig;
@group(2) @binding(1)
var atlas_sampler: sampler;
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var minmax_atlas: texture_2d_array<f32>;

#import bevy_terrain::node
#import bevy_terrain::functions

fn child_index() -> i32 {
    return atomicAdd(&parameters.child_index, parameters.counter);
}

fn parent_index(id: u32) -> i32 {
    return i32(view_config.tile_count - 1u) * clamp(parameters.counter, 0, 1) - i32(id) * parameters.counter;
}

fn final_index() -> i32 {
    return atomicAdd(&parameters.final_index, 1);
}

fn frustum_cull(tile: Tile) -> bool {
    let size = f32(tile.size) * view_config.tile_scale;
    let local_position = (vec2<f32>(tile.coords) + 0.5) * size;

    let minmax = vec2<f32>(0.0, config.height); // 2D frustum culling
    // Todo: enable this
    // let minmax = minmax(local_position, size); // 3D frustum culling

    // frustum culling optimized
    let aabb_min = vec3<f32>(local_position.x - size / 2.0, minmax.x, local_position.y - size / 2.0);
    let aabb_max = vec3<f32>(local_position.x + size / 2.0, minmax.y, local_position.y + size / 2.0);

    for (var i = 0; i < 5; i = i + 1) {
        let plane = view.planes[i];

        var p_corner = vec4<f32>(aabb_min.x, aabb_min.y, aabb_min.z, 1.0);
        var n_corner = vec4<f32>(aabb_max.x, aabb_max.y, aabb_max.z, 1.0);
        if (plane.x >= 0.0) { p_corner.x = aabb_max.x; n_corner.x = aabb_min.x; }
        if (plane.y >= 0.0) { p_corner.y = aabb_max.y; n_corner.y = aabb_min.y; }
        if (plane.z >= 0.0) { p_corner.z = aabb_max.z; n_corner.z = aabb_min.z; }

    	if (dot(plane, p_corner) < 0.0) {
    	    // the closest corner is outside the plane -> cull
    	    return true;
    	}
    	else if (dot(plane, n_corner) < 0.0) {
    	    // the furthest corner is inside the plane -> don't cull
    	    return false;
    	}
    }

    return false;
}

fn outside_cull(tile: Tile) -> bool {
    // cull tiles outside of the terrain
    let local_position = vec2<f32>(tile.coords * tile.size) * view_config.tile_scale ;

    return local_position.x > f32(config.terrain_size) || local_position.y > f32(config.terrain_size);
}

fn cull(tile: Tile) -> bool {
    return outside_cull(tile) || frustum_cull(tile);
}

fn should_be_divided(tile: Tile) -> bool {
    if (tile.size == 1u) {
        return false;
    }

    var dist = 1000000000.0;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let corner_coords = vec2<u32>(tile.coords.x + (i       & 1u),
                                      tile.coords.y + (i >> 1u & 1u));

        let local_position = vec2<f32>(corner_coords * tile.size) * view_config.tile_scale;
        let world_position = approximate_world_position(local_position);
        dist = min(dist, distance(world_position.xyz, view.world_position.xyz));
    }

    return dist < view_config.morph_distance * f32(tile.size);
}

fn subdivide(tile: Tile) {
    let size = tile.size >> 1u;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let coords = vec2<u32>((tile.coords.x << 1u) + (i       & 1u),
                               (tile.coords.y << 1u) + (i >> 1u & 1u));

        let tile = Tile(coords, size);

        if (!cull(tile)) {
            temporary_tiles.data[child_index()] = tile;
        }
    }
}

@compute @workgroup_size(64, 1, 1)
fn refine_tiles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    if (invocation_id.x >= parameters.tile_count) {
        return;
    }

    let tile = temporary_tiles.data[parent_index(invocation_id.x)];

    if (should_be_divided(tile)) {
        subdivide(tile);
    }
    else {
        final_tiles.data[final_index()] = tile;
    }
}
