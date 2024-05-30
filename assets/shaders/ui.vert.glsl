#version 460

layout(location = 0) in vec3 position;

layout(location = 0) out vec3 outPosition;
layout(location = 1) out vec4 outColour;
layout(location = 2) out vec4 outArea;
layout(location = 3) out float outRadius;
layout(location = 4) out vec2 outTexcoord;

struct Rectangle {
    vec4 colour;
    vec4 area;
    vec4 sample_area;
    float radius;
};

layout(set = 0, binding = 0) readonly buffer RectangleBuffer {
    Rectangle rectangles[];
} rectangles;

layout(set = 0, binding = 1) uniform Viewport {
    vec2 viewport;
} viewport;

void main() {
    uint index = uint(gl_VertexIndex / 4);
    Rectangle rectangle = rectangles.rectangles[index];
    
    gl_Position = vec4(((position.xy / viewport.viewport) - 0.5) * 2.0, position.z, 1.0);
    outPosition = position;
    
    outColour = rectangle.colour;
    outArea = rectangle.area;
    outRadius = rectangle.radius;

    uint n = gl_VertexIndex % 4;
    vec2 sample_offset = rectangle.sample_area.xy;
    vec2 sample_size = rectangle.sample_area.zw;
    if (n == 0) {
        outTexcoord = sample_offset;
    } else if (n == 1) {
        outTexcoord = sample_offset + vec2(sample_size.x, 0.0);
    } else if (n == 2) {
        outTexcoord = sample_offset + sample_size;
    } else if (n == 3) {
        outTexcoord = sample_offset + vec2(0.0, sample_size.y);
    }
}
