#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_types
#import bevy_terrain::config
#import bevy_terrain::tile

struct TerrainConfig {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
    _padding: u32,
    height_scale: f32,
    density_scale: f32,
    albedo_scale: f32,
    _empty: u32,
    height_offset: f32,
    density_offset: f32,
    albedo_offset: f32,
    _empty: u32,
}

// terrain view bindings
@group(1) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(1) @binding(1)
var quadtree: texture_2d_array<u32>;
@group(1) @binding(2)
var<storage> tiles: TileList;

// terrain bindings
@group(2) @binding(0)
var<uniform> config: TerrainConfig;
@group(2) @binding(1)
var filter_sampler: sampler;
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var density_atlas: texture_2d_array<f32>;
#ifdef ALBEDO
@group(2) @binding(4)
var albedo_atlas: texture_2d_array<f32>;
#endif


// mesh bindings
@group(3) @binding(0)
var<uniform> mesh: Mesh;

#import bevy_pbr::pbr_types
#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::pbr_functions

#import bevy_terrain::terrain
#import bevy_terrain::atlas
#import bevy_terrain::debug

fn height_vertex(atlas_index: i32, atlas_coords: vec2<f32>) -> f32 {
    let height_coords = atlas_coords * config.height_scale + config.height_offset;
    return config.height * textureSampleLevel(height_atlas, filter_sampler, height_coords, atlas_index, 0.0).x;
}

fn color_fragment(
    in: FragmentInput,
    lod: u32,
    atlas_index: i32,
    atlas_coords: vec2<f32>
) -> vec4<f32> {
    var color = vec4<f32>(0.0);

    let height_coords = atlas_coords * config.height_scale + config.height_offset;
    let albedo_coords = atlas_coords * config.albedo_scale + config.albedo_offset;

    #ifndef BRIGHT
        color = mix(color, vec4<f32>(1.0), 0.5);
    #endif

    #ifdef SHOW_LOD
        color = mix(color, show_lod(lod, in.world_position.xyz), 0.4);
    #endif

    #ifdef ALBEDO
        color = mix(color, textureSample(albedo_atlas, filter_sampler, albedo_coords, atlas_index), 0.5);
    #endif

    #ifdef SHOW_UV
        color = mix(color, vec4<f32>(atlas_coords.x, atlas_coords.y, 0.0, 1.0), 0.5);
    #endif

    #ifdef LIGHTING
        let world_normal = calculate_normal(height_coords, atlas_index, lod);

        // let ambient = 0.3;
        // let direction = normalize(vec3<f32>(3.0, 1.0, -2.0));
        // let diffuse = max(dot(direction, world_normal), 0.0);
        // color = color * (ambient + diffuse);

        var pbr_input: PbrInput = pbr_input_new();
        pbr_input.material.base_color = color;
        pbr_input.frag_coord = in.frag_coord;
        pbr_input.world_position = in.world_position;
        pbr_input.world_normal = world_normal;
        pbr_input.is_orthographic = view.projection[3].w == 1.0;
        pbr_input.N = world_normal;
        pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);
        color = pbr(pbr_input);
    #endif

    return color;
}

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var tile_lod = 0u;
    for (; tile_lod < 4u; tile_lod = tile_lod + 1u) {
        if (vertex.index < tiles.counts[tile_lod].y) {
            break;
        }
    }

    let tile_size = calc_tile_count(tile_lod);
    let vertices_per_row = (tile_size + 2u) << 1u;
    let vertices_per_tile = vertices_per_row * tile_size;

    let tile_index  = (vertex.index - tiles.counts[tile_lod].x) / vertices_per_tile + tile_lod * 100000u;
    let vertex_index = (vertex.index - tiles.counts[tile_lod].x) % vertices_per_tile;

    let tile = tiles.data[tile_index];
    let local_position = calculate_position(vertex_index, tile, vertices_per_row, tile_size);

    let world_position = vec3<f32>(local_position.x, view_config.height_under_viewer, local_position.y);
    let blend = calculate_blend(world_position, view_config.vertex_blend);

    let lookup = atlas_lookup(blend.log_distance, local_position);
    var height = height_vertex(lookup.atlas_index, lookup.atlas_coords);

    if (blend.ratio < 1.0) {
        let lookup2 = atlas_lookup(blend.log_distance + 1.0, local_position);
        var height2 = height_vertex(lookup2.atlas_index, lookup2.atlas_coords);
        height = mix(height2, height, blend.ratio);
    }

    var output = vertex_output(local_position, height);

#ifdef SHOW_TILES
    output.color = show_tiles(tile, local_position, tile_lod);
#endif

    return output;
}

@fragment
fn fragment(fragment: FragmentInput) -> FragmentOutput {
    let blend = calculate_blend(fragment.world_position.xyz, view_config.fragment_blend);

    let lookup = atlas_lookup(blend.log_distance, fragment.local_position);
    var color = color_fragment(fragment, lookup.lod, lookup.atlas_index, lookup.atlas_coords);

    if (blend.ratio < 1.0) {
        let lookup2 = atlas_lookup(blend.log_distance + 1.0, fragment.local_position);
        let color2 = color_fragment(fragment, lookup2.lod, lookup2.atlas_index, lookup2.atlas_coords);
        color = mix(color2, color, blend.ratio);
    }

    color = mix(fragment.color, color, 0.8);

    return FragmentOutput(color);
}
