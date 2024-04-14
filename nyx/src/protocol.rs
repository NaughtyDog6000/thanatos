use glam::Vec3;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ClientId(pub usize);

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub enum Clientbound {
    SetToken(ClientId),
    Spawn(ClientId, Vec3),
    Despawn(ClientId),
    Move(ClientId, Vec3)
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub enum Serverbound {
    Move(ClientId, Vec3),
    Disconnect
}
