use glam::{Vec2, Vec3, Vec4};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraint<T> {
    pub min: T,
    pub max: T,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub origin: Vec2,
    pub size: Vec2,
}

impl Area {
    pub fn points(&self) -> [Vec2; 4] {
        [
            self.origin,
            self.origin + Vec2::X * self.size.x,
            self.origin + self.size,
            self.origin + Vec2::Y * self.size.y,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    area: Area,
    radius: f32,
    colour: Vec4,
}

#[derive(Default)]
pub struct Layer {
    rectangles: Vec<Rectangle>,
}

pub struct Scene {
    layers: Vec<Layer>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            layers: vec![Layer::default()],
        }
    }

    pub fn rectangle(&mut self, rectangle: Rectangle) {
        self.layers.last_mut().unwrap().rectangles.push(rectangle)
    }

    pub fn vertices(&self) -> (Vec<Vec3>, Vec<usize>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for (depth, layer) in self.layers.iter().enumerate() {
            for rectangle in &layer.rectangles {
                indices.append(
                    &mut [0, 1, 2, 2, 3, 0]
                        .into_iter()
                        .map(|x| x + vertices.len())
                        .collect(),
                );

                vertices.extend_from_slice(
                    &rectangle
                        .area
                        .points()
                        .into_iter()
                        .map(|point| Vec3::new(point.x, point.y, depth as f32))
                        .collect::<Vec<_>>(),
                );
            }
        }

        (vertices, indices)
    }
}

pub trait Element {
    fn layout(&mut self, constraint: Constraint<Vec2>) -> Vec2;
    fn paint(&mut self, area: Area, scene: &mut Scene);
}

pub struct Box {}

impl Element for Box {
    fn layout(&mut self, constraint: Constraint<Vec2>) -> Vec2 {
        constraint.max
    }

    fn paint(&mut self, area: Area, scene: &mut Scene) {
        scene.rectangle(Rectangle {
            area,
            colour: Vec4::ONE,
            radius: 16.0
        })
    }
}
