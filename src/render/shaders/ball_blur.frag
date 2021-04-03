#version 450
const float EPSILON = 0.0001;
const float aa_pixels = 2.;

layout(location = 0) in vec2 coords;
layout(location = 1) in vec3 color;
layout(location = 2) in float trail_length;
layout(location = 3) in float total_portion;

layout(location = 0) out vec4 f_color;

float correct_value(float val, float d){
    if (val - d < 0){
        return (val+d)/2;
    }
    return val;
}

void main() {
    // The goal of anti aliasing is estimating the lit area, and sampling the color at the middle of
    // that area (i.e. average color).
    // For the second bit, we make sure to correct the values sampled.
    // For the first, we compute the area with the 'factor' logic.
    
    // In our case, the color is seg.
    // At the top/bottom, 
    float d2 = 1-coords.y*coords.y;
    float dwidth = length(vec2(dFdx(d2), dFdy(d2)));
    d2 = correct_value(d2, dwidth*0.5*aa_pixels);

    float d = sqrt(max(0,d2));
    float t0 = max(0, coords.x-d);
    float t1 = min(trail_length, coords.x+d);
    // Note that seg reaches negative value at the sides.
    float seg = t1 - t0;
    float xwidth = length(vec2(dFdx(coords.x), dFdy(coords.x)));
    seg = min(correct_value(seg, xwidth*0.5*aa_pixels), trail_length);
    float normalized_length = (seg+EPSILON)/(trail_length+EPSILON)*total_portion;
    float alpha = clamp(normalized_length, 0, 1);
    // alpha=seg;

    float ex = coords.x-clamp(coords.x, 0, trail_length);
    float dist = sqrt(ex*ex + coords.y*coords.y);
    float pwidth = length(vec2(dFdx(dist), dFdy(dist)));
    float factor = smoothstep(-0.5*aa_pixels, 0.5*aa_pixels, (1-dist)/pwidth);
    // alpha = factor;
    alpha *= factor;
    f_color = vec4(color, alpha);
}