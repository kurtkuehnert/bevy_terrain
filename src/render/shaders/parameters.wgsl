#define_import_path bevy_terrain::parameters

struct Parameters {
    lod: u32;
    child_index: atomic<u32>;
    final_index: atomic<u32>;
};
