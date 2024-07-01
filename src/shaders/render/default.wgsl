#import bevy_terrain::vertex::{VertexInput, VertexOutput, vertex_default}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_default}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    return vertex_default(input);
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    return fragment_default(input);
}
