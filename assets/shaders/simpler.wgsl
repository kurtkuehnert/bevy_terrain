




struct FragmentData {
    world_normal: vec3<f32>,
    color: vec4<f32>,
}



//how can i import this from the frag file ?
 
 
 
struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec2<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

// The processed fragment consisting of the color and a flag whether or not to discard this fragment.
struct Fragment {
    color: vec4<f32>,
    do_discard: bool,
}
@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
  
  let color = vec4<f32>(1.0, 1.0, 1.0, 1.0);

    return FragmentOutput(color);
    
    
}