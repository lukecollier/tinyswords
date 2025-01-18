#import bevy_sprite::mesh2d_view_bindings::globals 
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;

const PIXELATE_BY: f32 = 32.0;


fn random2(p: vec2<f32>) -> vec2<f32> {
    return fract(sin(vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)))) * 43758.5453);
}

    
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let time: f32 = globals.time;
    let resolution = view.viewport.zw;
    var uv: vec2<f32> = in.uv;
    // taken from https://thebookofshaders.com/12/
    // var st: vec2<f32> = (in.position.xy / resolution.xy) + uv;
    var color: vec3<f32> = vec3<f32>(0.);
    let base_color: vec3<f32> = vec3<f32>(99.0 / 255, 169.0 / 255, 168.0 / 255);
    let wave_color: vec3<f32> = vec3<f32>(120.0 / 255, 180.0 / 255, 180.0 / 255);

    // Scale
    uv *= 100.;

    // Tile the space
    let i_uv: vec2<f32> = floor(floor(uv) * 4.) / 4.;
    let f_uv: vec2<f32> = floor(fract(uv) * 4.) / 4.;

    var m_dist: f32 = 1.0;  // minimum distance

    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
             // Neighbor place in the grid
            let neighbor: vec2<f32> = vec2<f32>(f32(x), f32(y));

             // Random position from current + neighbor place in the grid
            var point: vec2<f32> = random2(i_uv + neighbor);

            // Animate the point
            point = 0.5 + 0.5 * sin((floor(time * 8.) / 8.) + 6.2831 * point);

            // Vector between the pixel and the point
            let diff: vec2<f32> = neighbor + point - f_uv;

            // Distance to the point
            let dist: f32 = length(diff);

             // Keep the closer distance
            m_dist = min(m_dist, dist);
        }
    }

    // could we infact do 3 levels of this, so the tips would be white
    // the middle darker
    // and the edge lightest
    // or even better do we do two worley noises moving randomly in opposite directions with a darker texture
    // then whenever they meet is white? to simulate waves of the ocean
    let wider = pow(m_dist, 10.);
    color += pow(m_dist, 20.);
    color *= wave_color;
    color = color / 5;
    color += base_color;


    return vec4<f32>(color.x, color.y, color.z, 1.);
}
