#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 color;

layout(push_constant) uniform Constants {
    float t;
    vec2 res;
    float aspect;
    vec2 mpos;
    vec2 mclick;
} c;

layout(set = 0, binding = 0) uniform Params {
    int spokes;
    float swirl;
    float cutoff;
    float speed;
    vec3 col;
} u;

float aa_step(float thres, float x) {
    float dx = length(vec2(dFdx(x), dFdy(x)));
    return smoothstep(thres-dx, thres+dx, x);
}

void main() {
    vec2 st = vec2(length(uv), atan(uv.y, uv.x));

    float v = aa_step(0, sin(4*u.swirl/st.x + u.spokes*st.y + 8*u.speed*c.t))
        * smoothstep(1-u.cutoff, 1-u.cutoff + 0.7, st.x);

    color = vec4(u.col, 1.0) * vec4(vec3(v), 1.0);
}
