#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 fragColor;

layout(set = 0, binding = 0) uniform Camera {
    mat4 viewProj;
} camera;

layout(set = 1, binding = 0) uniform Transform {
    mat4 transform;
} transform;

layout(set = 1, binding = 1) uniform Material {
    vec4 colour;
} material;

void main() {
    gl_Position = camera.viewProj * transform.transform * vec4(position, 1.0);
    fragColor = material.colour.rgb * (0.5 + 0.5 * max(dot(normal, vec3(1.0)), 0.0));
}
