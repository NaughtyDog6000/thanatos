use std::{collections::HashSet, sync::Arc, time::Duration};

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
}

impl Mouse {
    pub fn is_down(&self, button: MouseButton) -> bool {
        self.down.contains(&button)
    }

    pub fn tick(world: &World) {
        let mut mouse = world.get_mut::<Mouse>().unwrap();
        mouse.delta = Vec2::ZERO;
    }
}

#[derive(Clone, Default)]
pub struct Keyboard {
    down: HashSet<Key>,
}

impl Keyboard {
    pub fn is_down<T: IntoKey>(&self, key: T) -> bool {
        self.down.get(&key.into_key()).is_some()
    }
}

pub trait IntoKey {
    fn into_key(self) -> Key;
}

impl IntoKey for &str {
    fn into_key(self) -> Key {
        Key::Character(SmolStr::new_inline(self))
    }
}

impl IntoKey for Key {
    fn into_key(self) -> Key {
        self
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
            let mut mouse = world.get_mut::<Mouse>().unwrap();

            window
                .event_loop
                .pump_events(Some(Duration::ZERO), |event, _| {
                    match event {
                        winit::event::Event::WindowEvent { event, .. } => match event {
                            WindowEvent::Resized(new_size) => {
                                events.push(Event::Resized(new_size));
                            }
                            WindowEvent::CloseRequested => {
                                events.push(Event::Stop);
                            }
                            WindowEvent::KeyboardInput { event, .. } => {
                                match event.state {
                                    ElementState::Pressed => {
                                        keyboard.down.insert(event.logical_key.clone());
                                        events.push(Event::KeyPress(event.logical_key));
                                    }
                                    ElementState::Released => {
                                        keyboard.down.remove(&event.logical_key);
                                        events.push(Event::KeyRelease(event.logical_key));
                                    }
                                }
                            }
                            WindowEvent::MouseInput { state, button, .. } => {
                                match state {
                                    ElementState::Pressed => {
                                        mouse.down.insert(button);
                                        events.push(Event::MousePress(button))
                                    }
                                    ElementState::Released => {
                                        mouse.down.remove(&button);
                                        events.push(Event::MouseRelease(button))
                                    }
                                }
                            }
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
                        }
                        _ => ()
                    }
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
