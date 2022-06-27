#define_import_path bevy_terrain::parameters

struct Parameters {
    counter: i32;
    child_index: atomic<i32>;
    final_index: atomic<i32>;
};
