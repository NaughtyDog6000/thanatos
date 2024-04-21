use glam::Vec3;

use crate::item::ItemStack;

pub const TPS: f32 = 20.0;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ClientId(pub u64);
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Tick(pub u64);

impl Tick {
    pub fn inc(&mut self) {
        self.0 += 1
    }
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub enum Clientbound {
    AuthSuccess(ClientId),
    Spawn(ClientId, Vec3),
    Despawn(ClientId),
    Move(ClientId, Vec3, Tick),
    SetStack(ItemStack)
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub enum Serverbound {
    AuthRequest,
    Move(Vec3, Tick),
    Disconnect,
    Craft(usize),
    Gather(usize)
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ClientboundBundle {
    pub tick: Tick,
    pub messages: Vec<Clientbound>
}
