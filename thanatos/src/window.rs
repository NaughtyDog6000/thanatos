use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use glam::Vec2;
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, SmolStr},
    platform::pump_events::EventLoopExtPumpEvents,
    window::WindowBuilder,
};

use crate::{event::Event, World};

#[derive(Clone, Default)]
pub struct Mouse {
    pub position: Vec2,
    pub delta: Vec2,
    down: HashSet<MouseButton>,
    previous: HashSet<MouseButton>,
}

impl Mouse {
    pub fn is_down(&self, button: MouseButton) -> bool {
        self.down.contains(&button)
    }

    pub fn pressed(&self, button: MouseButton) -> bool {
        self.down.contains(&button) && !self.previous.contains(&button)
    }

    pub fn released(&self, button: MouseButton) -> bool {
        !self.down.contains(&button) && self.previous.contains(&button)
    }

    pub fn tick(world: &World) {
        let mut mouse = world.get_mut::<Mouse>().unwrap();
        mouse.delta = Vec2::ZERO;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Keybind {
    Interact,
}

#[derive(Clone)]
pub struct Keyboard {
    previous: HashSet<Key>,
    down: HashSet<Key>,
    pub keybinds: HashMap<Keybind, Key>,
}

impl Default for Keyboard {
    fn default() -> Self {
        let mut keyboard = Self {
            previous: HashSet::new(),
            down: HashSet::new(),
            keybinds: HashMap::new(),
        };
        keyboard.keybinds = HashMap::from([(Keybind::Interact, "f".into_key(&keyboard))]);
        keyboard
    }
}

impl Keyboard {
    pub fn pressed<T: IntoKey>(&self, key: T) -> bool {
        let key = key.into_key(self);
        self.down.contains(&key) && !self.previous.contains(&key)
    }

    pub fn released<T: IntoKey>(&self, key: T) -> bool {
        let key = key.into_key(self);
        !self.down.contains(&key) && self.previous.contains(&key)
    }

    pub fn is_down<T: IntoKey>(&self, key: T) -> bool {
        self.down.contains(&key.into_key(self))
    }
}

pub trait IntoKey {
    fn into_key(self, keyboard: &Keyboard) -> Key;
}

impl IntoKey for &str {
    fn into_key(self, _: &Keyboard) -> Key {
        Key::Character(SmolStr::new_inline(self))
    }
}

impl IntoKey for Key {
    fn into_key(self, _: &Keyboard) -> Key {
        self
    }
}

impl IntoKey for Keybind {
    fn into_key(self, keyboard: &Keyboard) -> Key {
        keyboard.keybinds.get(&self).unwrap().clone()
    }
}

pub struct Window {
    event_loop: EventLoop<()>,
    pub window: Arc<winit::window::Window>,
}

impl Window {
    pub fn new() -> Self {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        let window = WindowBuilder::new()
            .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
            .build(&event_loop)
            .unwrap();
        let window = Arc::new(window);
        Self { event_loop, window }
    }

    pub fn tick(world: &World) {
        let mut events = Vec::new();

        {
            let mut window = world.get_mut::<Window>().unwrap();
            let mut keyboard = world.get_mut::<Keyboard>().unwrap();
            keyboard.previous = keyboard.down.clone();
            let mut mouse = world.get_mut::<Mouse>().unwrap();
            mouse.previous = mouse.down.clone();

            window
                .event_loop
                .pump_events(Some(Duration::ZERO), |event, _| match event {
                    winit::event::Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(new_size) => {
                            events.push(Event::Resized(new_size));
                        }
                        WindowEvent::CloseRequested => {
                            events.push(Event::Stop);
                        }
                        WindowEvent::KeyboardInput { event, .. } => match event.state {
                            ElementState::Pressed => {
                                keyboard.down.insert(event.logical_key.clone());
                                events.push(Event::KeyPress(event.logical_key));
                            }
                            ElementState::Released => {
                                keyboard.down.remove(&event.logical_key);
                                events.push(Event::KeyRelease(event.logical_key));
                            }
                        },
                        WindowEvent::MouseInput { state, button, .. } => match state {
                            ElementState::Pressed => {
                                mouse.down.insert(button);
                                events.push(Event::MousePress(button))
                            }
                            ElementState::Released => {
                                mouse.down.remove(&button);
                                events.push(Event::MouseRelease(button))
                            }
                        },
                        WindowEvent::CursorMoved { position, .. } => {
                            let position = Vec2::new(position.x as f32, position.y as f32);
                            mouse.delta = position - mouse.position;
                            mouse.position = position;
                            events.push(Event::MouseMove {
                                position,
                                delta: mouse.delta,
                            })
                        }
                        _ => (),
                    },
                    _ => (),
                });
        }

        events.into_iter().for_each(|event| world.submit(event));
    }

    pub fn add(self) -> impl FnOnce(World) -> World {
        move |world| {
            world
                .with_resource(self)
                .with_resource(Mouse::default())
                .with_resource(Keyboard::default())
                .with_ticker(Mouse::tick)
                .with_ticker(Self::tick)
        }
    }

    pub fn screen_to_ndc(&self, pos: Vec2) -> Vec2 {
        let size = self.window.inner_size();
        pos * 2.0 / Vec2::new(size.width as f32, size.height as f32) - 1.0
    }
}
