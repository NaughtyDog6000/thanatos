use std::default;

use glam::Vec3;
use tecs::EntityId;

use crate::World;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn translate(&mut self, translation: Vec3) {
        self.origin += translation;
    }

    pub fn from_points(from: Vec3, to: Vec3) -> Self {
        Self {
            origin: from,
            direction: (to - from).normalize(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ColliderKind {
    Sphere(f32),
    Aabb(Vec3),
}

#[derive(Clone, Copy, Debug)]
pub enum ColliderPositionKind {
    Absolute(Vec3),
    Relative(Vec3, EntityId), //offset, parent transform
}

impl Default for ColliderPositionKind {
    fn default() -> Self {
        Self::Absolute(Vec3 { x: 0.0, y: 0.0, z: 0.0 })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Collider {
    pub kind: ColliderKind,
    pub position: ColliderPositionKind,
}

impl Collider {
    pub fn within(&self, point: Vec3, world: &World) -> bool {
        let point = point - self.calculate_position(world);

        match self.kind {
            ColliderKind::Sphere(radius) => point.length() < radius,
            ColliderKind::Aabb(size) => point
                .to_array()
                .into_iter()
                .zip(size.to_array().into_iter())
                .all(|(distance, max_distance)| distance.abs() < max_distance),
        }
    }

    fn calculate_position(&self, world: &World) -> Vec3 {
        return match self.position {
            ColliderPositionKind::Absolute(pos) => pos,
            ColliderPositionKind::Relative(rel_pos, parent_id) => {
                let x = world.get_component::<crate::transform::Transform>(parent_id).map(|t| *t).unwrap_or_default();
                x.translation + rel_pos
            }
        };
    }


    fn quadratic(a: f32, b: f32, c: f32) -> Option<(f32, f32)> {
        let discriminant = b * b - 4.0 * a * c;
        println!("{}", discriminant);
        if discriminant < 0.0 {
            return None;
        }
        let x1 = (-b + discriminant.sqrt()) / (2.0 * a);
        let x2 = (-b - discriminant.sqrt()) / (2.0 * a);
        Some((x1, x2))
    }

    pub fn intersects(&self, mut ray: Ray, world: &World) -> Option<Vec3> {
        let calculated_position = self.calculate_position(world);
        
        ray.translate(-calculated_position);

        match self.kind {
            ColliderKind::Sphere(radius) => {
                let a = ray.direction.length_squared();
                let b = 2.0 * ray.origin.dot(ray.direction);
                let c = ray.origin.length_squared() - radius.powi(2);
                Self::quadratic(a, b, c)
                    .map(|(t1, t2)| t1.min(t2))
                    .map(|t| ray.origin + ray.direction * t)
                    .map(|pos| pos + calculated_position)
            }
            ColliderKind::Aabb(size) => {
                let ts = size
                    .to_array()
                    .iter()
                    .zip(&ray.origin.to_array())
                    .zip(ray.direction.to_array())
                    .map(|((size, origin), direction)| {
                        let t1 = (size - origin) / direction;
                        let t2 = (-size - origin) / direction;
                        (t1, t2)
                    })
                    .collect::<Vec<_>>();

                let tmin = ts
                    .iter()
                    .map(|(t1, t2)| t1.min(*t2))
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();

                let tmax = ts
                    .iter()
                    .map(|(t1, t2)| t1.max(*t2))
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();

                if tmax < 0.0 {
                    return None;
                }
                if tmin > tmax {
                    return None;
                }

                let t = if tmin < 0.0 { tmax } else { tmin };
                Some(ray.origin + ray.direction * t + calculated_position)
            }
        }
    }
}
