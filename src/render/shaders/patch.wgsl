#define_import_path bevy_terrain::patch

struct Patch {
    coords: vec2<u32>;
    size: u32;
    stitch: u32;
    morph: u32;
    parent_count: u32;
};

struct PatchList {
    counts: array<vec2<u32>, 4>;
    data: array<Patch>;
};

fn calc_patch_size(lod: u32) -> u32 {
    return (lod + 1u) << 1u; // 2, 4, 6, 8, ...
    // return 2u << lod; // 2, 4, 8, 16, ...
}