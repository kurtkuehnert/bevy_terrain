
const {attachment_name_upper}_SIZE  : f32 = {attachment_size:.10};
const {attachment_name_upper}_SCALE : f32 = {attachment_scale:.10};
const {attachment_name_upper}_OFFSET: f32 = {attachment_offset:.10};

@group(2) @binding({binding})
var {attachment_name_lower}_atlas: texture_2d_array<f32>;
