use hord3::horde::game_engine::{entity::{Component, ComponentEvent, StaticComponent}, multiplayer::Identify};
use to_from_bytes_derive::{FromBytes, ToBytes};

use crate::game_entity::actions::{Action, ActionResult};
#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct Director {
    finished_actions:Vec<(Action, ActionResult)>,
    kind:DirectorKind
}

impl Director {
    pub fn new(kind:DirectorKind) -> Self {
        Self { finished_actions: Vec::with_capacity(4), kind }
    }
}

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub enum DirectorKind {
    Player,
    Nothing
}

#[derive(Clone, PartialEq)]
pub struct StaticDirector {
    pub kind:DirectorKind
}

impl StaticComponent for StaticDirector {

}
#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct DirectorEvent<ID:Identify> {
    id:usize,
    source:Option<ID>,
    update:DirectorUpdate
}

impl<ID:Identify> DirectorEvent<ID> {
    pub fn new(id:usize, source:Option<ID>, update:DirectorUpdate) -> Self {
        Self { id, source, update }
    }
}

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub enum DirectorUpdate {
    FlushFinished,
    NotifyFinished((Action, ActionResult)),
    ChangeKind(DirectorKind)
}

impl<ID:Identify> ComponentEvent<Director, ID> for DirectorEvent<ID> {
    type ComponentUpdate = DirectorUpdate;
    fn get_id(&self) -> hord3::horde::game_engine::entity::EntityID {
        self.id
    }
    fn get_source(&self) -> Option<ID> {
        self.source.clone()
    }
    fn apply_to_component(self, components:&mut Vec<Director>) {
        match self.update {
            DirectorUpdate::FlushFinished => components[self.id].finished_actions.clear(),
            DirectorUpdate::ChangeKind(new_kind) => components[self.id].kind = new_kind,
            DirectorUpdate::NotifyFinished(finished) => components[self.id].finished_actions.push(finished),
        }
    }
}

impl<ID:Identify> Component<ID> for Director {
    type CE = DirectorEvent<ID>;
    type SC = StaticDirector;
    fn from_static(static_comp:&Self::SC) -> Self {
        Self { finished_actions: Vec::new(), kind: static_comp.kind.clone() }
    }
}

