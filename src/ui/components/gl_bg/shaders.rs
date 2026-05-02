pub const MESH_VERTEX_SHADER: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texCoord;
layout(location = 2) in vec3 a_color;

uniform float u_aspectRatio;

out vec2 v_texCoord;
out vec3 v_color;

void main() {
    vec2 pos = a_position;
    
    // 宽高比修正
    if (u_aspectRatio > 1.0) {
        pos.y *= u_aspectRatio;
    } else {
        pos.x /= u_aspectRatio;
    }
    
    gl_Position = vec4(pos * 1.15, 0.0, 1.0);
    
    v_texCoord = a_texCoord;
    v_color = a_color;
}
"#;
pub const MESH_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;
in vec3 v_color;
in vec2 v_texCoord;
uniform sampler2D u_texture;
uniform float u_time;
uniform float u_volume;
uniform float u_alpha;
uniform int u_has_texture;
out vec4 fragColor;

const float INV_255 = 1.0 / 255.0;
const float HALF_INV_255 = 0.5 / 255.0;
const float GRADIENT_NOISE_A = 52.9829189;
const vec2 GRADIENT_NOISE_B = vec2(0.06711056, 0.00583715);

float gradientNoise(in vec2 uv) {
    return fract(GRADIENT_NOISE_A * fract(dot(uv, GRADIENT_NOISE_B)));
}

vec2 rot(vec2 v, float angle) {
    float s = sin(angle);
    float c = cos(angle);
    return vec2(c * v.x - s * v.y, s * v.x + c * v.y);
}

void main() {
    float volumeEffect = u_volume * 2.0;
    float timeVolume = u_time + u_volume;

    float dither = INV_255 * gradientNoise(gl_FragCoord.xy) - HALF_INV_255;
    vec2 centeredUV = v_texCoord - vec2(0.2);
    vec2 rotatedUV = rot(centeredUV, timeVolume * 2.0);
    vec2 finalUV = rotatedUV * max(0.001, 1.0 - volumeEffect) + vec2(0.5);

    vec4 result;
    if (u_has_texture != 0) {
        result = texture(u_texture, finalUV);
    } else {
        result = vec4(1.0);
    }

    float alphaVolumeFactor = u_alpha * max(0.5, 1.0 - u_volume * 0.5);
    result.rgb *= v_color * alphaVolumeFactor;
    result.a *= alphaVolumeFactor;

    result.rgb += vec3(dither);

    float dist = distance(v_texCoord, vec2(0.5));
    float vignette = smoothstep(0.8, 0.3, dist);
    float mask = 0.6 + vignette * 0.4;
    result.rgb *= mask;

    fragColor = vec4(clamp(result.rgb, 0.0, 1.0), clamp(result.a, 0.0, 1.0));
}
"#;
pub const QUAD_VERTEX_SHADER: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texCoord;
out vec2 v_texCoord;
void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
    v_texCoord = a_texCoord;
}
"#;

pub const QUAD_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;
in vec2 v_texCoord;
uniform sampler2D u_texture;
uniform float u_alpha;
out vec4 fragColor;
void main() {
    vec4 color = texture(u_texture, v_texCoord);
    fragColor = vec4(color.rgb, color.a * u_alpha);
}
"#;