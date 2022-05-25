#define_import_path bevy_terrain::patch

struct Patch {
    coords: vec2<u32>;
    size: u32;
    stitch: u32; // 4 bit
};

struct PatchList {
    data: array<Patch>;
};
