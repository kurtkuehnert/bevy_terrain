#define_import_path bevy_terrain::patch

struct Patch {
    coords: vec2<u32>;
    size: u32;
    stitch: u32;
    morph: u32;
    lod_diff: u32;
};

struct PatchList {
    counts: array<vec2<u32>, 4>;
    data: array<Patch>;
};
