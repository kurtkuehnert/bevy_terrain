#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput
#import bevy_terrain::vertex vertex_fn
#import bevy_terrain::fragment fragment_fn

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    return vertex_fn(in);
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    return fragment_fn(in);
}
