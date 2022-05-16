#define_import_path bevy_terrain::patch

struct Patch {
    position: vec2<u32>;
    scale: u32;
    lod_delta: u32; // should be u16
};

struct PatchList {
    data: array<Patch>;
};
