// shader.vert
#version 440

layout(location=0) in vec2 a_ndc;
layout(location=1) in vec3 a_xyz;

layout(location=0) out vec2 pos_cs;
layout(location=1) out vec3 pos_xyz;

layout(set = 0, binding = 3)
uniform Window {
    vec4 size;
};

void main() {
    gl_Position = vec4(vec2(a_ndc.x * size.x, a_ndc.y * size.y), 0.0, 1.0);
    pos_cs = a_ndc*0.5 + 0.5;
    pos_xyz = a_xyz;
}