use anyhow::Result;
use glam::Vec3;
use nyx::protocol::{ClientId, Clientbound, Serverbound};
use std::{
    io::{ErrorKind, Read, Write},
    net::TcpStream,
};
use tecs::{impl_archetype, Is};
use thanatos_macros::Archetype;

use crate::{
    assets::{self, MaterialId, MeshId},
    event::Event,
    renderer::RenderObject,
    transform::Transform,
    World,
};

pub struct Connection {
    tcp: TcpStream,
    buffer: Vec<u8>,
    pub id: Option<ClientId>,
}

impl Connection {
    pub fn new() -> Result<Self> {
        let tcp = TcpStream::connect("127.0.0.1:8080")?;
        tcp.set_nonblocking(true).unwrap();
        Ok(Self {
            tcp,
            buffer: Vec::new(),
            id: None,
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

    fn get(&mut self) -> Option<Clientbound> {
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
        let mut conn = world.get_mut::<Connection>().unwrap();
        conn.read().unwrap();

        let Some(message) = conn.get() else { return };
        println!("Recieved: {:?}", message);
        match message {
            Clientbound::SetToken(id) => conn.id = Some(id),
            message => world.submit(Event::Recieved(message)),
        };
    }

    pub fn add(world: World) -> World {
        world
            .with_resource(Self::new().unwrap())
            .with_ticker(Self::tick)
    }
}

#[derive(Archetype)]
pub struct OtherPlayer {
    pub client_id: ClientId,
    pub render: RenderObject,
    pub transform: Transform,
}

impl OtherPlayer {
    pub fn handle_net(mesh: MeshId, material: MaterialId) -> impl Fn(&World, &Event) {
        move |world, event| match event {
            Event::Recieved(message) => match message {
                Clientbound::Spawn(client_id, position) => {
                    let render = RenderObject { mesh, material };
                    let mut transform = Transform::IDENTITY;
                    transform.translation = *position;
                    world.spawn(OtherPlayer {
                        client_id: *client_id,
                        render,
                        transform,
                    });
                }
                Clientbound::Move(client_id, position) => {
                    let (mut transforms, client_ids, _) =
                        world.query::<(&mut Transform, &ClientId, Is<OtherPlayer>)>();
                    let mut n = client_ids
                        .iter()
                        .position(|other| *client_id == *other)
                        .unwrap() as i64;
                    transforms.for_each(|transform| {
                        if n == 0 {
                            transform.translation = *position
                        };
                        n -= 1
                    })
                }
                Clientbound::Despawn(client_id) => {
                    let (mut transforms, client_ids, _) = world.query::<(&mut Transform, &ClientId, Is<OtherPlayer>)>();
                    let mut n = client_ids
                        .iter()
                        .position(|other| *client_id == *other)
                        .unwrap() as i64;
                    transforms.for_each(|transform| {
                        if n == 0 {
                            transform.translation = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
                        };
                        n -= 1
                    })
                }
                _ => (),
            },
            _ => (),
        }
    }

    pub fn add(mesh: MeshId, material: MaterialId) -> impl FnOnce(World) -> World {
        move |world| {
            world
                .register::<OtherPlayer>()
                .with_handler(Self::handle_net(mesh, material))
        }
    }
}
