#import bevy_sprite::mesh2d_view_bindings
#import bevy_pbr::utils

@group(1) @binding(0)
var texture: texture_2d<f32>;

@group(1) @binding(1)
var _sampler: sampler;

@group(1) @binding(2)
var<uniform> pixel_block_size: f32;

@group(1) @binding(3)
var<uniform> chromatic_aberration_intensity: f32;

@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    #import bevy_sprite::mesh2d_vertex_output
) -> @location(0) vec4<f32> {
    var uv = coords_to_viewport_uv(position.xy, view.viewport);
    let resolution = vec2<f32>(textureDimensions(texture));

    let width_height_over_block_size = resolution / max(1.0, pixel_block_size);

    uv *= width_height_over_block_size;
    uv = floor(uv);
    uv /= width_height_over_block_size;

    var output_color = vec4<f32>(
        textureSample(texture, _sampler, uv + vec2<f32>(0.0, -chromatic_aberration_intensity)).r,
        textureSample(texture, _sampler, uv + vec2<f32>(-chromatic_aberration_intensity, 0.0)).g,
        textureSample(texture, _sampler, uv + vec2<f32>(0.0, chromatic_aberration_intensity)).b,
        1.0
    );

    return output_color;
}