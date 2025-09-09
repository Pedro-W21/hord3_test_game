use std::{collections::HashSet, sync::LazyLock};

use hord3::horde::game_engine::{entity::{Component, ComponentEvent, StaticComponent}, multiplayer::Identify, world::WorldComputeHandler};
use to_from_bytes_derive::{FromBytes, ToBytes};

use crate::{game_engine::{CoolGameEngineTID, CoolVoxel, ExtraData}, game_entity::{actions::{Action, ActionCounter, ActionResult}, director::llm_director::LLMDirector, GameEntityVecRead}, game_map::{GameMap, WorldVoxelPos}, proxima_link::HordeProximaAIResponse};

pub mod llm_director;

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct Director {
    finished_actions:Vec<(Action, ActionResult)>,
    name:String,
    alerts:Vec<DirectorAlert>,
    kind:DirectorKind
}

static DEFAULT_NAMES:LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
    vec![
        format!("Jack"),
        format!("Nick"),
        format!("Natalie"),
        format!("John"),
        format!("Peter"),
        format!("Gargamel"),
        format!("Piccolo")
    ]
});

impl Director {
    pub fn new(kind:DirectorKind,name:String) -> Self {
        Self { finished_actions: Vec::with_capacity(4), kind, name, alerts:Vec::with_capacity(3)  }
    }
    pub fn new_with_random_name(kind:DirectorKind) -> Self {
        Self { finished_actions: Vec::with_capacity(4), kind, name:fastrand::choice(DEFAULT_NAMES.iter()).unwrap().clone(), alerts:Vec::with_capacity(3)  }
    }
    pub fn get_name(&self) -> &String {
        &self.name
    }
    pub fn do_tick<'a>(
        &self,
        agent_id:usize,
        first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>,
        tick:usize,
        counter:&mut ActionCounter,
    ) {
        match &self.kind {
            DirectorKind::LLM(llm_director) => {
                let mut new_director = llm_director.clone();
                new_director.parse_responses(agent_id, first_ent, second_ent, world, tick, counter);
                first_ent.tunnels.director_out.send(DirectorEvent::new(agent_id, None, DirectorUpdate::UpdateKind(DirectorKind::LLM(new_director))));
            },
            _ => ()
        }
        
    }
    pub fn do_after_tick<'a>(
        &self,
        agent_id:usize,
        first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>,
        extra_data:&ExtraData,
        tick:usize,
    ) {
        match &self.kind {
            DirectorKind::LLM(llm_director) => {
                let mut new_director = llm_director.clone();
                for alert in &self.alerts {
                    let payload = new_director.get_payload(agent_id, alert.clone(), first_ent, second_ent, world, tick);
                    extra_data.payload_sender.send(payload);
                }
                match new_director.get_periodic_payload(agent_id, first_ent, second_ent, world, tick) {
                    Some(payload) => {extra_data.payload_sender.send(payload);},
                    None => ()
                }
                first_ent.tunnels.director_out.send(DirectorEvent::new(agent_id, None, DirectorUpdate::UpdateKind(DirectorKind::LLM(new_director))));
                first_ent.tunnels.director_out.send(DirectorEvent::new(agent_id, None, DirectorUpdate::FlushAlerts));
            },
            _ => ()
        }
        
    }
}

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub enum DirectorAlert {
    Periodic,
    HeardWords(usize, String), // speaker ID, text
    FinishedMoveTo(WorldVoxelPos, bool) // moved to, failed/worked
}

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub enum DirectorKind {
    Player,
    LLM(LLMDirector),
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
    UpdateKind(DirectorKind),
    LLMAddToResponses(HordeProximaAIResponse),
    SendAlert(DirectorAlert),
    FlushAlerts
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
            DirectorUpdate::UpdateKind(new_kind) => components[self.id].kind = new_kind,
            DirectorUpdate::NotifyFinished(finished) => components[self.id].finished_actions.push(finished),
            DirectorUpdate::LLMAddToResponses(response) => match &mut components[self.id].kind {
                DirectorKind::LLM(director) => director.responses.push(response),
                _ => ()
            },
            DirectorUpdate::SendAlert(alert) => components[self.id].alerts.push(alert),
            DirectorUpdate::FlushAlerts => components[self.id].alerts.clear(),
        }
    }
}

impl<ID:Identify> Component<ID> for Director {
    type CE = DirectorEvent<ID>;
    type SC = StaticDirector;
    fn from_static(static_comp:&Self::SC) -> Self {
        Self { finished_actions: Vec::new(), kind: static_comp.kind.clone(), name:String::from("Placeholder"), alerts:Vec::new() }
    }
}

