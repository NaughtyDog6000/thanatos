use std::{
    cell::Cell,
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use anyhow::Result;
use async_std::{channel, task};
use async_std::{
    channel::{Receiver, Sender},
    io::WriteExt,
    net::{TcpListener, TcpStream},
};
use futures::{
    io::{ReadHalf, WriteHalf},
    AsyncReadExt,
};
use glam::Vec3;
use nyx::protocol::{ClientId, Clientbound, ClientboundBundle, Serverbound, Tick, TPS};

pub async fn read(reader: &mut ReadHalf<TcpStream>) -> Result<Serverbound> {
    let mut length_bytes = [0; 4];
    reader.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes) as usize;

    let mut buffer = vec![0_u8; length];
    reader.read_exact(&mut buffer).await?;
    Ok(bincode::deserialize(&buffer)?)
}

pub async fn write(writer: &mut WriteHalf<TcpStream>, bundle: ClientboundBundle) -> Result<()> {
    let buffer = bincode::serialize(&bundle)?;
    writer
        .write_all(&(buffer.len() as u32).to_be_bytes())
        .await?;
    writer.write_all(&buffer).await?;
    Ok(())
}

pub struct Client {
    position: Cell<Vec3>,
    tx: Sender<Clientbound>,
    rx: Receiver<Serverbound>,
}

#[async_std::main]
async fn main() -> Result<()> {
    let mut clients: HashMap<ClientId, Client> = HashMap::new();
    let (clients_tx, clients_rx) = channel::unbounded();
    let (flush_tx, flush_rx) = channel::unbounded();

    task::spawn(async move {
        let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
        let mut next = 0;
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let (clientbound_tx, clientbound_rx) = channel::unbounded();
            let (serverbound_tx, serverbound_rx) = channel::unbounded();

            let id = ClientId(next);
            let client = Client {
                position: Cell::new(Vec3::ZERO),
                rx: serverbound_rx,
                tx: clientbound_tx,
            };
            next += 1;
            clients_tx.send((id, client)).await.unwrap();

            let (mut reader, mut writer) = stream.split();

            task::spawn(async move {
                loop {
                    let message = match read(&mut reader).await {
                        Ok(message) => message,
                        Err(_) => Serverbound::Disconnect,
                    };

                    let serverbound_tx = serverbound_tx.clone();

                    task::spawn(async move {
                        task::sleep(Duration::from_millis(100)).await;
                        println!("Received {:?} from {id:?}", message);
                        serverbound_tx.send(message).await.unwrap();
                    });
                }
            });

            let flush_rx = flush_rx.clone();

            task::spawn(async move {
                let mut messages = Vec::new();
                loop {
                    if let Ok(message) = clientbound_rx.try_recv() {
                        println!("Sending {:?} to {id:?}", message);
                        messages.push(message);
                    }

                    if let Ok(tick) = flush_rx.try_recv() {
                        println!("Flushing {:?}", tick);
                        write(&mut writer, ClientboundBundle { tick, messages })
                            .await
                            .unwrap();
                        messages = Vec::new();
                    }
                }
            });
        }
    });

    let mut tick = Tick(0);
    let mut last_tick = Instant::now();

    loop {
        if last_tick.elapsed() < Duration::from_secs_f32(1.0 / TPS) {
            continue;
        }
        last_tick = Instant::now();
        println!("Handling: {tick:?}");

        while let Ok((id, client)) = clients_rx.try_recv() {
            println!("Adding {:?} to clients", id);

            client.tx.send(Clientbound::AuthSuccess(id)).await.unwrap();
            for (other_id, other) in clients.iter() {
                other
                    .tx
                    .send(Clientbound::Spawn(id, client.position.get()))
                    .await
                    .unwrap();
                client
                    .tx
                    .send(Clientbound::Spawn(*other_id, other.position.get()))
                    .await
                    .unwrap();
            }

            clients.insert(id, client);
        }

        let mut to_remove = Vec::new();

        for (id, client) in clients.iter() {
            while let Ok(message) = client.rx.try_recv() {
                match message {
                    Serverbound::Move(client_id, position, tick) => {
                        if *id != client_id {
                            println!("ID mismatch");
                            continue;
                        }
                        client.position.set(position);
                        client
                            .tx
                            .send(Clientbound::Move(*id, client.position.get(), tick))
                            .await
                            .unwrap();
                        for (_, other) in clients.iter().filter(|(other_id, _)| *other_id != id) {
                            other
                                .tx
                                .send(Clientbound::Move(*id, client.position.get(), tick))
                                .await
                                .unwrap();
                        }
                    }
                    Serverbound::Disconnect => {
                        to_remove.push(*id);
                    }
                }
            }
        }

        to_remove.iter().for_each(|id| {
            clients.remove(id);
        });

        for id in to_remove.drain(..) {
            for (_, client) in clients.iter() {
                client.tx.send(Clientbound::Despawn(id)).await.unwrap()
            }
        }

        flush_tx.send(tick).await.unwrap();
        tick.inc();
    }
}
