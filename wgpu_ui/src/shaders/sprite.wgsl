struct Cam {
    zx: f32,
    zy: f32,
    cx: f32,
    cy: f32,
};
@group(0) @binding(0) var<uniform> cam: Cam;
@group(1) @binding(0) var samp: sampler;
@group(1) @binding(1) var tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    // Y-вниз: clip.y инвертируется (см. camera::gpu_uniform).
    out.pos = vec4<f32>(p.x * cam.zx - cam.cx, -(p.y * cam.zy - cam.cy), 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return in.color * textureSample(tex, samp, in.uv);
}
