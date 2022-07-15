#define_import_path bevy_terrain::parameters

struct Parameters {
    counter: i32,
    child_index: atomic<i32>,
    final_index1: atomic<i32>,
    final_index2: atomic<i32>,
    final_index3: atomic<i32>,
    final_index4: atomic<i32>,
    // final_indices:  array<atomic<i32>, 4>;
}
