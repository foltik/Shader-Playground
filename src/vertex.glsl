#version 450

layout(location = 0) out vec2 uv;

layout(push_constant) uniform Constants {
    layout (offset = 16) float aspect;
} c;

void main() {
    // Generate a single fullscreen quad out of a single larger triangle
    vec2 tex = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
    gl_Position = vec4(tex*2 - 1, 0, 1);

    // Center UVs in [-0.5, 0.5]
    uv = tex - 0.5;

    // Extend UVs past 0.5 in the larger dimension
    if (c.aspect > 1)
        uv.x *= c.aspect;
    else
        uv.y /= c.aspect;
}
