#define_import_path bevy_terrain::parameters

struct Parameters {
    child_index: atomic<u32>;
    final_index: atomic<u32>;
    patch_index: atomic<u32>;
    lod: u32;
    previous_node_count: u32;
    node_counts: array<u32, 16>;
};
