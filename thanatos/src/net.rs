use anyhow::Result;
use glam::{Vec3, Vec4};
use nyx::protocol::{ClientId, Clientbound, ClientboundBundle, Serverbound, Tick, TPS};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    io::ErrorKind,
    net::UdpSocket,
    time::{Duration, Instant},
};
use tecs::prelude::*;

use crate::{
    assets::{Material, MeshId},
    event::Event,
    player::Player,
    renderer::RenderObject,
    transform::Transform,
    World,
};

pub struct Connection {
    socket: UdpSocket,
    pub id: Option<ClientId>,
    pub tick: Tick,
}

impl Connection {
    pub fn new() -> Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.connect("127.0.0.1:8080")?;
        socket.set_nonblocking(true)?;
        let mut conn = Self {
            socket,
            id: None,
            tick: Tick(0),
        };
        conn.write(Serverbound::AuthRequest).unwrap();
        Ok(conn)
    }

    pub fn write(&mut self, message: Serverbound) -> Result<()> {
        let data = bincode::serialize(&message)?;
        self.socket.send(&data)?;
        Ok(())
    }

    fn get(&mut self) -> Option<ClientboundBundle> {
        let mut buffer = [0; 4096];
        match self.socket.recv(&mut buffer) {
            Ok(_) => Some(bincode::deserialize(&buffer).unwrap()),
            Err(e) if e.kind() == ErrorKind::WouldBlock => None,
            Err(e) => panic!("{e}"),
        }
    }

    pub fn tick(world: &World) {
        let messages: Vec<Clientbound> = {
            let mut conn = world.get_mut::<Connection>().unwrap();

            let Some(bundle) = conn.get() else { return };
            conn.tick = bundle.tick;
            println!("Received: {:?}", bundle.tick);
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

#[derive(Clone, Debug)]
pub struct Positions {
    queue: VecDeque<(Instant, Vec3)>,
}

impl Positions {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, position: Vec3) {
        self.queue.push_back((
            Instant::now() + Duration::from_secs_f32(2.0 / TPS),
            position,
        ))
    }

    pub fn get(&mut self) -> Option<Vec3> {
        let now = Instant::now();
        match self.queue.len() {
            0 => None,
            1 => self.queue.get(1).map(|x| x.1),
            _ => {
                let first = self.queue.front().unwrap();
                let second = self.queue.get(1).unwrap();
                if second.0 < now {
                    self.queue.pop_front();
                    self.get()
                } else {
                    let t = (now - first.0).as_secs_f32() / (second.0 - first.0).as_secs_f32();
                    Some(second.1 * t + first.1 * (1.0 - t))
                }
            }
        }
    }
}

#[derive(Archetype, Clone)]
pub struct OtherPlayer {
    pub client_id: ClientId,
    pub render: RenderObject,
    pub transform: Transform,
    pub positions: Positions,
}

pub struct MovementSystem {
    positions: RefCell<HashMap<Tick, Vec3>>,
}

impl MovementSystem {
    fn spawn(&self, world: &World, client_id: ClientId, position: Vec3) {
        let render = RenderObject {
            mesh: MeshId(String::from("assets/meshes/cube.glb")),
            material: Material { colour: Vec4::ONE },
        };
        let mut transform = Transform::IDENTITY;
        transform.translation = position;
        world.spawn(OtherPlayer {
            client_id,
            render,
            transform,
            positions: Positions::new(),
        });
    }

    fn move_player(&self, world: &World, position: Vec3, tick: Tick) {
        let (mut transform, _) = world.query_one::<(&mut Transform, Is<Player>)>();

        if let Some(actual) = self.positions.borrow().get(&tick) {
            if position == *actual {
                return;
            }
        }

        transform.translation = position;
    }

    fn move_other_player(&self, world: &World, client_id: ClientId, position: Vec3) {
        let (mut positions, client_ids, _) =
            world.query::<(&mut Positions, &ClientId, Is<OtherPlayer>)>();
        let mut n = client_ids
            .iter()
            .position(|other| client_id == *other)
            .unwrap() as i64;

        positions.for_each(|positions| {
            if n == 0 {
                positions.push(position);
            };
            n -= 1
        })
    }

    fn update_buffered_positions(world: &World) {
        let (mut transforms, mut positions) = world.query::<(&mut Transform, &mut Positions)>();
        let mut positions = positions.map(|position| position.get()).into_iter();
        transforms.for_each(|transform| {
            if let Some(position) = positions.next().unwrap() {
                transform.translation = position
            }
        });
    }

    fn despawn(&self, world: &World, client_id: ClientId) {
        let id = {
            let (entities, client_ids, _) = world.query::<(EntityId, &ClientId, Is<OtherPlayer>)>();
            entities
                .iter()
                .zip(client_ids.iter().collect::<Vec<_>>())
                .find(|(_, id)| **id == client_id)
                .map(|(entity, _)| *entity)
        };

        if let Some(id) = id {
            world.despawn::<OtherPlayer>(id);
        }
    }

    fn send_player_position(&self, world: &World) {
        let mut conn = world.get_mut::<Connection>().unwrap();
        let (transforms, _) = world.query::<(&Transform, Is<Player>)>();
        let position = transforms.iter().next().unwrap().translation;
        if conn.id.is_none() {
            return;
        }
        let tick = conn.tick;
        conn.write(Serverbound::Move(position, tick)).unwrap();
        self.positions.borrow_mut().insert(tick, position);
    }
}

impl System<Event> for MovementSystem {
    fn event(&self, world: &World, event: &Event) {
        match event {
            Event::Recieved(message) => match message {
                Clientbound::Spawn(client_id, position) => self.spawn(world, *client_id, *position),
                Clientbound::Move(client_id, position, tick) => {
                    println!("Moving {client_id:?} from {tick:?}");
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

pub fn add(mut world: World) -> World {
    world
        .register_unsaved::<OtherPlayer>()
        .with_system(MovementSystem {
            positions: RefCell::new(HashMap::new()),
        })
}
