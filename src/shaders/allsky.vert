// shader.vert
#version 440

layout(location=0) in vec2 a_position;
layout(location=0) out vec2 pos_cs;
layout(set = 0, binding = 5)
uniform Window {
    vec2 size;
};
layout(set = 0, binding = 6)
uniform Time {
    float time;
};

void main() {
    gl_Position = vec4(vec2(a_position.x * size.x, a_position.y * size.y), 0.0, 1.0);
    pos_cs = a_position*0.5 + 0.5;
}