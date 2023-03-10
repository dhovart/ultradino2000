#import bevy_sprite::mesh2d_view_bindings

@group(1) @binding(0)
var texture: texture_2d<f32>;

@group(1) @binding(1)
var _sampler: sampler;

@group(1) @binding(2)
var<uniform> time: f32;


struct FragmentInput {
    #import bevy_pbr::mesh_vertex_output
}

@fragment
fn fragment(
    in: FragmentInput
) -> @location(0) vec4<f32> {
  var uv = in.uv;
  let offset_x = sin(uv.y + time * 2. + sin(uv.x * 5.)) * 0.025;
  let offset_y = cos(uv.x + time * 2.1 + sin(uv.y * 4.)) * 0.045;
  var new_uv = vec2<f32>(uv.x + offset_x, uv.y + offset_y);
  return textureSample(texture, _sampler, new_uv);
}