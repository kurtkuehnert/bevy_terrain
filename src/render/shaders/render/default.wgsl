#import bevy_terrain::vertex VertexInput, VertexOutput, default_vertex
#import bevy_terrain::fragment FragmentInput, FragmentOutput, default_fragment

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    return default_vertex(input);
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    return default_fragment(input);
}
