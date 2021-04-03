#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 coords;
layout(location = 2) in vec3 color;
layout(location = 3) in float trail_length;
layout(location = 4) in float total_portion;

layout(location = 0) out vec2 outCoords;
layout(location = 1) out vec3 outColor;
layout(location = 2) out float out_trail_length;
layout(location = 3) out float out_total_portion;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    outCoords = coords;
    outColor = color;
    out_trail_length = trail_length;
    out_total_portion = total_portion;
}