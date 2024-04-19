#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 colour;
layout(location = 2) in vec4 area;
layout(location = 3) in float radius;
layout(location = 4) in vec2 texcoord;

layout(location = 0) out vec4 outColour;

layout(set = 0, binding = 2) uniform sampler2D atlas;

float rect_sdf(
    vec2 absolute_pixel_position,
    vec2 origin,
    vec2 size,
    float corner_radius
) {
    vec2 half_size = size / 2.;
    vec2 rect_center = origin + half_size;
 
    // Change coordinate space so that the rectangle's center is at the origin,
    // taking advantage of the problem's symmetry.
    vec2 pixel_position = abs(absolute_pixel_position - rect_center);
 
    // Shrink rectangle by the corner radius.
    vec2 shrunk_corner_position = half_size - corner_radius;
 
    // Determine the distance vector from the pixel to the rectangle corner,
    // disallowing negative components to simplify the three cases.
    vec2 pixel_to_shrunk_corner = max(vec2(0.0), pixel_position - shrunk_corner_position);
 
    float distance_to_shrunk_corner = length(pixel_to_shrunk_corner);
 
    // Subtract the corner radius from the calculated distance to produce a
    // rectangle having the desired size.
    float distance = distance_to_shrunk_corner - corner_radius;
 
    return distance;
}

void main() {
    float distance = rect_sdf(position.xy, area.xy, area.zw, radius);

    if (distance > 0.0) {
        outColour = vec4(0.0);
    } else if (texcoord == vec2(0.0)) {
        outColour = colour;
    } else {
        outColour = colour * textureLod(atlas, texcoord, 0).x;
    }
}

