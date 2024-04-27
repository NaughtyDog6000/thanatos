use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use glam::Vec3;
use nyx::{
    data,
    equipment::{Equipment, EquipmentId, EquipmentInventory, Passive},
    item::{Inventory, Item, ItemStack, LootTable, Rarity, RecipeOutput, RARITIES},
    protocol::{ClientId, Clientbound, ClientboundBundle, Serverbound, Tick, TPS},
};

const FORCED_LATENCY: Duration = Duration::from_millis(300);

pub struct Client {
    id: ClientId,
    position: Cell<Vec3>,
    inventory: RefCell<Inventory>,
    equipment: RefCell<EquipmentInventory>,
}

fn handle_networking(
    socket: UdpSocket,
    clientbound_rx: Receiver<(SocketAddr, Clientbound)>,
    flush_rx: Receiver<Tick>,
    serverbound_tx: Sender<(SocketAddr, Serverbound)>,
) {
    let mut buf = [0; 4096];
    println!("Listening");
    let mut messages: HashMap<SocketAddr, Vec<Clientbound>> = HashMap::new();
    let mut to_receive = VecDeque::new();
    loop {
        if let Ok((addr, message)) = clientbound_rx.try_recv() {
            match messages.get_mut(&addr) {
                Some(messages) => messages.push(message),
                None => {
                    messages.insert(addr, vec![message]);
                }
            }
        }

        if let Ok(tick) = flush_rx.try_recv() {
            messages.iter_mut().for_each(|(addr, messages)| {
                let bundle = ClientboundBundle {
                    tick,
                    messages: messages.to_vec(),
                };
                *messages = Vec::new();
                let buffer = bincode::serialize(&bundle).unwrap();
                socket.send_to(&buffer, addr).unwrap();
            })
        }

        let (n, addr) = match socket.recv_from(&mut buf) {
            Ok((n, addr)) => (n, addr),
            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => panic!("{e:?}"),
        };
        let Ok(message) = bincode::deserialize::<Serverbound>(&buf[0..n]) else {
            continue;
        };
        println!("{n} from {addr:?}");

        to_receive.push_back((Instant::now(), (addr, message)));
        while let Some((time, _)) = to_receive.get(0) {
            if *time + FORCED_LATENCY < Instant::now() {
                serverbound_tx
                    .send(to_receive.pop_front().unwrap().1)
                    .unwrap()
            } else {
                break;
            }
        }
    }
}

fn add_client(
    clients: &mut HashMap<SocketAddr, Client>,
    tx: &Sender<(SocketAddr, Clientbound)>,
    id: ClientId,
    addr: SocketAddr,
) -> Result<()> {
    tx.send((addr, Clientbound::AuthSuccess(id)))?;
    clients
        .iter()
        .map(|(other_addr, other)| {
            tx.send((*other_addr, Clientbound::Spawn(id, Vec3::ZERO)))?;
            tx.send((addr, Clientbound::Spawn(other.id, other.position.get())))?;
            Ok(())
        })
        .collect::<Result<Vec<_>>>()?;
    clients.insert(
        addr,
        Client {
            id,
            position: Cell::new(Vec3::ZERO),
            inventory: RefCell::new(Inventory::default()),
            equipment: RefCell::new(EquipmentInventory(Vec::new())),
        },
    );

    Ok(())
}

fn main() -> Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:8080").unwrap();
    socket.set_nonblocking(true).unwrap();
    let mut clients: HashMap<SocketAddr, Client> = HashMap::new();
    let (serverbound_tx, serverbound_rx) = unbounded();
    let (clientbound_tx, clientbound_rx) = unbounded();
    let (flush_tx, flush_rx) = unbounded();

    std::thread::spawn(|| handle_networking(socket, clientbound_rx, flush_rx, serverbound_tx));

    let mut next = 0;
    let mut next_equipment = 0;
    let mut tick = Tick(0);
    let rx = serverbound_rx;
    let tx = clientbound_tx;

    let recipes = data::recipes();
    let nodes = data::nodes();

    loop {
        let start = Instant::now();

        while let Ok((addr, message)) = rx.try_recv() {
            if let Serverbound::AuthRequest = message {
                let id = ClientId(next);
                add_client(&mut clients, &tx, id, addr).unwrap();
                next += 1;
            }

            let Some(client) = clients.get(&addr) else {
                continue;
            };
            match message {
                Serverbound::Move(position, tick) => {
                    let changed = client.position.get() != position;
                    client.position.set(position);
                    clients.keys().for_each(|other_addr| {
                        if *other_addr != addr && !changed {
                            return;
                        }
                        tx.send((*other_addr, Clientbound::Move(client.id, position, tick)))
                            .unwrap();
                    })
                }
                Serverbound::Gather(index) => {
                    let Some(node) = nodes.get(index) else {
                        continue;
                    };
                    let mut inventory = client.inventory.borrow_mut();
                    node.pick().iter().for_each(|stack| {
                        inventory.add(*stack);
                        tx.send((
                            addr,
                            Clientbound::SetStack(ItemStack {
                                item: stack.item,
                                quantity: inventory.get(stack.item).unwrap_or_default(),
                            }),
                        ))
                        .unwrap();
                    })
                }
                Serverbound::Craft(index, rarities) => {
                    let Some(recipe) = recipes.get(index) else {
                        continue;
                    };
                    let mut inventory = client.inventory.borrow_mut();
                    let mut equipment = client.equipment.borrow_mut();
                    if !recipe.craftable(&inventory.items().collect::<Vec<_>>(), &rarities) {
                        continue;
                    }
                    recipe.inputs.iter().cloned().zip(rarities.clone()).for_each(
                        |((kind, quantity), rarity)| {
                            let item = Item { kind, rarity };
                            inventory.remove(ItemStack { item, quantity });
                            tx.send((
                                addr,
                                Clientbound::SetStack(ItemStack {
                                    item,
                                    quantity: inventory.get(item).unwrap_or_default(),
                                }),
                            ))
                            .unwrap();
                        },
                    );

                    let chances = recipe.rarity_chances(&rarities);
                    let rarity = *RARITIES
                        .into_iter()
                        .zip(chances)
                        .fold(LootTable::default(), |picker, (rarity, chance)| {
                            picker.add(chance, rarity)
                        })
                        .pick();

                    match recipe.output {
                        RecipeOutput::Items(kind, quantity) => {
                            let item = Item {
                                kind,
                                rarity,
                            };
                            inventory.add(ItemStack { item, quantity });
                            tx.send((
                                addr,
                                Clientbound::SetStack(ItemStack {
                                    item,
                                    quantity: inventory.get(item).unwrap_or_default(),
                                }),
                            ))
                            .unwrap();
                        }
                        RecipeOutput::Equipment(kind) => {
                            let piece = Equipment {
                                id: EquipmentId(next_equipment),
                                kind,
                                rarity,
                                durability: 10,
                                passives: vec![Passive::FireDamage(0.2)],
                            };
                            next_equipment += 1;
                            equipment.0.push(piece.clone());
                            tx.send((addr, Clientbound::AddEquipment(piece))).unwrap();
                        }
                    }
                }
                _ => (),
            }
        }

        tick.0 += 1;
        flush_tx.send(tick).unwrap();
        std::thread::sleep(Duration::from_secs_f32(1.0 / TPS) - start.elapsed())
    }
}
