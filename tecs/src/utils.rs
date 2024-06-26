use std::{fmt::Display, time::{Duration, Instant}};

use serde::{Deserialize, Serialize};

use crate::World;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Timer {
    #[serde(skip)]
    start: Option<Instant>,
    pub duration: Duration,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            start: None,
            duration,
        }
    }

    pub fn start(&mut self) {
        self.start = Some(Instant::now())
    }

    pub fn done(&self) -> bool {
        self.start
            .map(|start| start.elapsed() > self.duration)
            .unwrap_or(true)
    }
}

#[derive(Clone, Debug)]
pub struct Clock {
    pub delta: Duration,
    pub start: Instant,
    last: Instant,
}

impl Clock {
    pub fn add<E: 'static>(world: World<E>) -> World<E> {
        world
            .with_resource(Self {
                delta: Duration::ZERO,
                start: Instant::now(),
                last: Instant::now(),
            })
            .with_ticker(Self::tick)
    }

    pub fn tick<E>(world: &World<E>) {
        let mut clock = world.get_mut::<Clock>().unwrap();
        let now = Instant::now();
        clock.delta = now - clock.last;
        clock.last = now;
    }
}

#[derive(Copy, Clone, Debug)]
pub enum State {
    Stopped,
    Running,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Name(pub String);

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
