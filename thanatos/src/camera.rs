use glam::{Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles};

use crate::{
    event::Event,
    window::{Mouse, Window},
    World,
};

pub struct Camera {
    pub target: Vec3,
    pub theta: f32,
    pub distance: f32,
    pub fov: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(window: &Window) -> Self {
        let size = window.window.inner_size();
        let aspect = size.width as f32 / size.height as f32;
        Self {
            target: Vec3::ZERO,
            theta: 0.0,
            distance: 10.0,
            fov: std::f32::consts::PI / 2.0,
            aspect,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let eye = Vec3::new(0.0, -1.0, -1.0).normalize() * self.distance;
        let rotated = Quat::from_rotation_y(self.theta) * eye;
        rotated + self.target
    }

    pub fn direction(&self) -> Vec3 {
        (self.eye() - self.target).normalize()
    }

    pub fn get_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Y);
        let projection = Mat4::perspective_infinite_rh(self.fov, self.aspect, 0.1);
        projection * view
    }

    pub fn ndc_to_world(&self, pos: Vec2) -> Vec3 {
        let transform = self.get_matrix().inverse();
        let transformed = transform * Vec4::new(pos.x, pos.y, 0.0, 1.0);
        transformed.xyz() / transformed.w
    }

    pub fn handle_resize(world: &World, event: &Event) {
        match event {
            Event::Resized(new_size) => {
                let mut camera = world.get_mut::<Camera>().unwrap();
                camera.aspect = new_size.width as f32 / new_size.height as f32;
            }
            _ => (),
        }
    }

    pub fn rotate_camera(world: &World) {
        let mouse = world.get::<Mouse>().unwrap();
        let mut camera = world.get_mut::<Camera>().unwrap();
        if mouse.is_down(winit::event::MouseButton::Right) {
            println!("{:?}", mouse.delta.x);
            camera.theta -= mouse.delta.x * 0.02;
        }
    }

    pub fn add(self) -> impl FnOnce(World) -> World {
        move |world| {
            world
                .with_resource(self)
                .with_handler(Self::handle_resize)
                .with_ticker(Self::rotate_camera)
        }
    }
}
