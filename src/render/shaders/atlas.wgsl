#define_import_path bevy_terrain::atlas

struct AtlasLookup {
    lod: u32;
    atlas_index: i32;
    atlas_coords: vec2<f32>;
};


fn atlas_lookup(world_position: vec2<f32>) -> AtlasLookup {
    let map_coords =  vec2<i32>(world_position / f32(config.chunk_size));
    let lookup = textureLoad(atlas_map, map_coords, 0);

    var output: AtlasLookup;

    output.lod = lookup.x;
    output.atlas_index = i32(lookup.y * 256u + lookup.z);
    output.atlas_coords = (world_position / f32(config.chunk_size * (1u << output.lod))) % 1.0;

    return output;
}