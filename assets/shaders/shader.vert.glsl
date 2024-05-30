#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 fragColor;

layout(set = 0, binding = 0) uniform Camera {
    mat4 viewProj;
} camera;

struct Transform {
    mat4 transform;
};

struct Material {
    vec4 colour;
};

layout(set = 1, binding = 0) readonly buffer Transforms {
    Transform transforms[];
} transforms;

layout(set = 1, binding = 1) readonly buffer Materials {
    Material materials[];
} materials;

void main() {
    uint index = gl_DrawID;
    Transform transform = transforms.transforms[index];
    Material material = materials.materials[index];
    
    gl_Position = camera.viewProj * transform.transform * vec4(position, 1.0);
    fragColor = material.colour.rgb * (0.5 + 0.5 * max(dot(normal, vec3(1.0)), 0.0));
}
