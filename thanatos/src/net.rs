use anyhow::Result;
use glam::Vec3;
use nyx::protocol::{ClientId, Clientbound, ClientboundBundle, Serverbound, Tick, TPS};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::{HashMap, VecDeque},
    io::{ErrorKind, Read, Write},
    net::TcpStream,
    time::Instant,
};
use tecs::{impl_archetype, Is, System};
use thanatos_macros::Archetype;

use crate::{
    assets::{self, MaterialId, MeshId},
    event::Event,
    player::Player,
    renderer::RenderObject,
    transform::Transform,
    World,
};

pub struct Connection {
    tcp: TcpStream,
    buffer: Vec<u8>,
    pub id: Option<ClientId>,
    pub tick: Tick,
}

impl Connection {
    pub fn new() -> Result<Self> {
        let tcp = TcpStream::connect("127.0.0.1:8080")?;
        tcp.set_nonblocking(true).unwrap();
        Ok(Self {
            tcp,
            buffer: Vec::new(),
            id: None,
            tick: Tick(0),
        })
    }

    pub fn write(&mut self, message: Serverbound) -> Result<()> {
        let data = bincode::serialize(&message)?;
        let mut buffer = (data.len() as u32).to_be_bytes().to_vec();
        buffer.extend_from_slice(&data);
        self.tcp.write_all(&buffer)?;
        Ok(())
    }

    fn read(&mut self) -> Result<()> {
        let mut buffer = vec![0; 2048];
        loop {
            match self.tcp.read(&mut buffer) {
                Ok(n) => self.buffer.extend_from_slice(&buffer[0..n]),
                Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn get(&mut self) -> Option<ClientboundBundle> {
        if self.buffer.len() < 4 {
            return None;
        }
        let length = u32::from_be_bytes(self.buffer[0..4].try_into().unwrap()) as usize;
        if self.buffer.len() < length + 4 {
            return None;
        }
        let message = bincode::deserialize(&self.buffer[4..length + 4]).ok()?;
        self.buffer.drain(0..length + 4);
        Some(message)
    }

    pub fn tick(world: &World) {
        let messages: Vec<Clientbound> = {
            let mut conn = world.get_mut::<Connection>().unwrap();
            conn.read().unwrap();

            let Some(bundle) = conn.get() else { return };
            conn.tick = bundle.tick;
            bundle
                .messages
                .into_iter()
                .filter(|message| match message {
                    Clientbound::AuthSuccess(id) => {
                        conn.id = Some(*id);
                        false
                    }
                    _ => true,
                })
                .collect()
        };
        messages
            .into_iter()
            .for_each(|message| world.submit(Event::Recieved(message)));
        world.submit(Event::ServerTick);
    }

    pub fn add(world: World) -> World {
        world
            .with_resource(Self::new().unwrap())
            .with_ticker(Self::tick)
    }
}

pub struct PositionBuffer {
    updated: Instant,
    last: Vec3,
    current: Vec3,
}

impl PositionBuffer {
    pub fn new(position: Vec3) -> Self {
        Self {
            last: position,
            current: position,
            updated: Instant::now(),
        }
    }

    pub fn tick(&mut self, item: Option<Vec3>) {
        println!("Updated: {item:?}");
        self.updated = Instant::now();
        self.last = self.current;
        self.current = item.unwrap_or(self.current);
    }

    pub fn get(&self) -> Vec3 {
        let t = self.updated.elapsed().as_secs_f32() * TPS;
        let t = t.min(1.0);
        self.current * t + self.last * (1.0 - t)
    }
}

#[derive(Archetype)]
pub struct OtherPlayer {
    pub client_id: ClientId,
    pub render: RenderObject,
    pub transform: Transform,
    pub positions: PositionBuffer,
}

pub struct MovementSystem {
    mesh: MeshId,
    material: MaterialId,
    positions: RefCell<HashMap<Tick, Vec3>>,
}

impl MovementSystem {
    fn spawn(&self, world: &World, client_id: ClientId, position: Vec3) {
        let render = RenderObject {
            mesh: self.mesh,
            material: self.material,
        };
        let mut transform = Transform::IDENTITY;
        transform.translation = position;
        world.spawn(OtherPlayer {
            client_id,
            render,
            transform,
            positions: PositionBuffer::new(position),
        });
    }

    fn move_player(&self, world: &World, position: Vec3, tick: Tick) {
        let (mut transforms, _) = world.query::<(&mut Transform, Is<Player>)>();

        if let Some(actual) = self.positions.borrow().get(&tick) {
            if position == *actual {
                return;
            }
        }

        transforms.for_each(|transform| transform.translation = position);
    }

    fn move_other_player(&self, world: &World, client_id: ClientId, position: Vec3) {
        let (mut positions, client_ids, _) =
            world.query::<(&mut PositionBuffer, &ClientId, Is<OtherPlayer>)>();
        let mut n = client_ids
            .iter()
            .position(|other| client_id == *other)
            .unwrap() as i64;

        positions.for_each(|positions| {
            if n == 0 {
                positions.tick(Some(position));
            };
            n -= 1
        })
    }

    fn update_buffered_positions(world: &World) {
        let (mut transforms, positions) = world.query::<(&mut Transform, &PositionBuffer)>();
        let mut positions = positions.iter();
        transforms.for_each(|transform| transform.translation = positions.next().unwrap().get());
    }

    fn despawn(&self, world: &World, client_id: ClientId) {
        let (mut transforms, client_ids, _) =
            world.query::<(&mut Transform, &ClientId, Is<OtherPlayer>)>();
        let mut n = client_ids
            .iter()
            .position(|other| client_id == *other)
            .unwrap() as i64;
        transforms.for_each(|transform| {
            if n == 0 {
                transform.translation = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
            };
            n -= 1
        })
    }

    fn send_player_position(&self, world: &World) {
        let mut conn = world.get_mut::<Connection>().unwrap();
        let (transforms, _) = world.query::<(&Transform, Is<Player>)>();
        let position = transforms.iter().next().unwrap().translation;
        if conn.id.is_none() {
            return;
        }
        let (client_id, tick) = (conn.id.unwrap(), conn.tick);
        conn.write(Serverbound::Move(client_id, position, tick))
            .unwrap();
        self.positions.borrow_mut().insert(tick, position);
    }
}

impl System<Event> for MovementSystem {
    fn event(&self, world: &World, event: &Event) {
        match event {
            Event::Recieved(message) => match message {
                Clientbound::Spawn(client_id, position) => self.spawn(world, *client_id, *position),
                Clientbound::Move(client_id, position, tick) => {
                    let conn = world.get::<Connection>().unwrap();
                    if *client_id == conn.id.unwrap() {
                        self.move_player(world, *position, *tick);
                    } else {
                        self.move_other_player(world, *client_id, *position);
                    }
                }
                Clientbound::Despawn(client_id) => self.despawn(world, *client_id),
                _ => (),
            },
            Event::ServerTick => self.send_player_position(world),
            _ => (),
        }
    }

    fn tick(&self, world: &World) {
        Self::update_buffered_positions(world); 
    }
}

pub fn add(mesh: MeshId, material: MaterialId) -> impl FnOnce(World) -> World {
    move |world| {
        world.register::<OtherPlayer>().with_system(MovementSystem {
            mesh,
            material,
            positions: RefCell::new(HashMap::new()),
        })
    }
}
