use std::{collections::{HashMap, HashSet}, sync::{LazyLock, OnceLock}};

use hord3::horde::{game_engine::{entity::{Component, ComponentEvent, StaticComponent}, multiplayer::Identify, world::WorldComputeHandler}, geometry::vec3d::{Vec3D, Vec3Df}};
use to_from_bytes_derive::{FromBytes, ToBytes};

use crate::{game_engine::{CoolGameEngineTID, CoolVoxel}, game_entity::{actions::{Action, ActionCounter, ActionResult, ActionSource, ActionTimer}, GameEntityVecRead}, game_map::{get_voxel_pos, GameMap}};

const DIRECTIONS:[Vec3D<i32> ; 12] = [
    Vec3D::new(1, 0, 0),
    Vec3D::new(0, 1, 0),
    Vec3D::new(-1, 0, 0),
    Vec3D::new(0, -1, 0),
    Vec3D::new(1, 0, 1),
    Vec3D::new(0, 1, 1),
    Vec3D::new(-1, 0, 1),
    Vec3D::new(0, -1, 1),
    Vec3D::new(1, 0, -1),
    Vec3D::new(0, 1, -1),
    Vec3D::new(-1, 0, -1),
    Vec3D::new(0, -1, -1),
];

static DIRECTIONS_INDICES:LazyLock<HashSet<usize>> = LazyLock::new(|| {HashSet::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])});

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct Planner {
    finished_actions:Vec<(Action, ActionResult)>,
    plans:Vec<Plan>
}

impl Planner {
    pub fn new() -> Self {
        Self { finished_actions: Vec::with_capacity(4), plans: Vec::with_capacity(4) }
    }
    pub fn update<'a>(
        &self,
        agent_id:usize,
        extra_possible_iterations:usize,
        first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>
    ) {
        if self.finished_actions.len() > 0 {

            first_ent.tunnels.planner_out.send(PlannerEvent::new(agent_id, None, PlannerUpdate::FlushFinished));
            for (finished, result) in &self.finished_actions {
                match self.get_plan_for_id(finished.get_id()) {
                    Some(plan) => {first_ent.tunnels.planner_out.send(PlannerEvent::new(agent_id, None, PlannerUpdate::RemovePlanAssociatedTo(plan.plan_action_id)));},
                    None => ()
                }
            }
        }
        for plan in &self.plans {
            if !plan.finished_compute() {
                match &plan.plan_data {
                    PlanData::Pathfinding(path) => {
                        let mut new_path = path.clone();
                        new_path.reiterate(agent_id, extra_possible_iterations, first_ent, second_ent, world);
                        let mut new_plan = plan.clone();
                        new_plan.plan_data = PlanData::Pathfinding(new_path);
                        first_ent.tunnels.planner_out.send(PlannerEvent::new(agent_id, None, PlannerUpdate::UpdatePlan(new_plan)));
                    }
                }
            }
        }
    }
    pub fn get_plan_for_id(&self, action_id:usize) -> Option<&Plan> {
        self.plans.iter().find(|plan| {plan.plan_action_id == action_id})
    }
    pub fn plan_exists_for(&self, action_id:usize) -> bool {
        self.plans.iter().find(|plan| {plan.plan_action_id == action_id}).is_some()
    }
}

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct Plan {
    plan_action_id:usize,
    plan_data:PlanData
}

impl Plan {
    pub fn finished_compute(&self) -> bool {
        match &self.plan_data {
            PlanData::Pathfinding(path) => path.found_path.is_some()
        }
    }
    pub fn create_pathfinding<'a>(
        action_id:usize,
        tolerance:f32,
        start_pos:Vec3Df, 
        end_pos:Vec3Df, 
        agent_id:usize,
        max_iterations:usize,
        first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>
    ) -> Plan {
        Plan { plan_action_id: action_id, plan_data: PlanData::Pathfinding(PathfindingData::plan_pathfinding(tolerance, start_pos, end_pos, agent_id, max_iterations, first_ent, second_ent, world)) }
    }
    pub fn get_actions_to_add(&self, counter:&mut ActionCounter, started_at:usize) -> Option<Vec<Action>> {
        match &self.plan_data {
            PlanData::Pathfinding(path_data) => if let Some(path) = &path_data.found_path {
                let mut actions = Vec::with_capacity(path.len());
                for node in path {
                    let id = counter.get_next_id();
                    let pos = path_data.nodes[*node].position;
                    actions.push(Action::new(id, started_at, ActionTimer::Delay(500), super::actions::ActionKind::MoveTowards(Vec3Df::new(pos.x as f32, pos.y as f32, pos.z as f32), path_data.tolerance), ActionSource::Planner));
                }
                let id = counter.get_next_id();
                
                actions.push(Action::new(id, started_at, ActionTimer::Delay(500), super::actions::ActionKind::StopAt(path_data.end_pos, 0.02, path_data.tolerance), ActionSource::Planner));
                // dbg!(actions.clone());
                // panic!("");
                Some(actions)
            }
            else {
                None
            }, 
        }
    }
}

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct PathfindingData {
    nodes:Vec<PathNode>,
    explored_positions:HashSet<Vec3D<i32>>,
    start_pos:Vec3Df,
    tolerance:f32,
    end_pos:Vec3Df,
    end_pos_i:Vec3D<i32>,
    iterations:usize,
    last_node:usize,
    found_path:Option<Vec<usize>>
}

pub fn default_heuristic(test:Vec3D<i32>, target:Vec3D<i32>) -> i32 {
    (test.x - target.x).pow(2) + (test.y - target.y).pow(2) + (test.z - target.z).pow(2)
}

impl PathfindingData {
    pub fn plan_pathfinding<'a>(
        tolerance:f32,
        start_pos:Vec3Df, 
        end_pos:Vec3Df, 
        agent_id:usize,
        max_iterations:usize,
        first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>
    ) -> PathfindingData {
        let mut data = PathfindingData {
            nodes:Vec::with_capacity(256),
            explored_positions:HashSet::with_capacity(512),
            iterations:0,
            start_pos,
            tolerance,
            end_pos,
            end_pos_i:get_voxel_pos(end_pos),
            last_node:0,
            found_path:None
        };
        let start_pos_vox = get_voxel_pos(start_pos);
        data.explored_positions.insert(start_pos_vox);
        data.nodes.push(PathNode { parent: None, position: start_pos_vox, tried_directions: HashSet::with_capacity(DIRECTIONS.len()) });

        while data.iterations < max_iterations && data.found_path.is_none() {
            data.pathfinding_iteration(agent_id, max_iterations, first_ent, second_ent, world);
        }
        
        data
    }
    fn reiterate<'a>(
        &mut self,
        agent_id:usize,
        extra_iterations:usize,
        first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>
    ) {
        let iters = self.iterations;
        let new_max = iters + extra_iterations;
        while self.iterations < new_max && self.found_path.is_none() {
            self.pathfinding_iteration(agent_id, new_max, first_ent, second_ent, world);
        }
    }
    fn pathfinding_iteration<'a>(
        &mut self,
        agent_id:usize,
        max_iterations:usize,
        first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>
    ) {
        let node = &mut self.nodes[self.last_node];
        self.iterations += 1;
        let remaining_directions:Vec<&usize> = DIRECTIONS_INDICES.difference(&node.tried_directions).collect();
        if remaining_directions.len() > 0 {
            let mut best_dir = 0;
            let mut best_dir_heuristic = 10000000;
            for dir in remaining_directions {
                let new_pos = node.position + DIRECTIONS[*dir];
                if !self.explored_positions.contains(&new_pos) {
                    let heuristic = default_heuristic(new_pos, self.end_pos_i);
                    if heuristic < best_dir_heuristic {
                        best_dir = *dir;
                        best_dir_heuristic = heuristic;
                    }
                }
                
            }
            let best_pos = node.position + DIRECTIONS[best_dir];
            node.tried_directions.insert(best_dir);
            self.explored_positions.insert(best_pos);
            if !world.world.is_voxel_solid(best_pos) && world.world.is_voxel_solid(best_pos + Vec3D::new(0, 0, -1)) {
                let new_last = self.nodes.len();
                self.nodes.push(PathNode { parent: Some(self.last_node), position: best_pos, tried_directions: HashSet::with_capacity(12) });
                self.last_node = new_last;
                if best_dir_heuristic == 0 {
                    self.create_path();
                }
            }
        }
        else if let Some(parent) = node.parent {
            self.last_node = parent;
        }
    }
    fn create_path(&mut self) {
        let mut path = Vec::with_capacity(40);
        path.push(self.last_node);
        let mut node = &mut self.nodes[self.last_node];
        while let Some(parent) = node.parent {
            path.push(parent);
            node = &mut self.nodes[parent];
        }
        path.reverse();
        self.found_path = Some(path)
    }
}
#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct PathNode {
    parent:Option<usize>,
    position:Vec3D<i32>,
    tried_directions:HashSet<usize>
}
#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub enum PlanData {
    Pathfinding(PathfindingData)
}  

#[derive(Clone, PartialEq)]
pub struct StaticPlanner {

}

impl StaticComponent for StaticPlanner {

}
#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct PlannerEvent<ID:Identify> {
    id:usize,
    source:Option<ID>,
    update:PlannerUpdate
}

impl<ID:Identify> PlannerEvent<ID> {
    pub fn new(id:usize, source:Option<ID>, update:PlannerUpdate) -> Self {
        Self { id, source, update }
    }
}


#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub enum PlannerUpdate {
    FlushFinished,
    FlushPlans,
    AddPlan(Plan),
    UpdatePlan(Plan),
    AddFinished((Action, ActionResult)),
    RemovePlanAssociatedTo(usize)
}

impl<ID:Identify> ComponentEvent<Planner, ID> for PlannerEvent<ID> {
    type ComponentUpdate = PlannerUpdate;
    fn get_id(&self) -> hord3::horde::game_engine::entity::EntityID {
        self.id
    }
    fn get_source(&self) -> Option<ID> {
        self.source.clone()
    }
    fn apply_to_component(self, components:&mut Vec<Planner>) {
        match self.update {
            PlannerUpdate::FlushFinished => components[self.id].finished_actions.clear(),
            PlannerUpdate::FlushPlans => components[self.id].plans.clear(),
            PlannerUpdate::AddFinished(action) => components[self.id].finished_actions.push(action),
            PlannerUpdate::AddPlan(plan) => components[self.id].plans.push(plan),
            PlannerUpdate::UpdatePlan(new_plan) => {components[self.id].plans.iter_mut().enumerate().find(|(i,plan)| {plan.plan_action_id == new_plan.plan_action_id}).and_then(|(i, plan)| {Some(i)}).and_then(|i| {components[self.id].plans[i] = new_plan; Some(0)});},
            PlannerUpdate::RemovePlanAssociatedTo(action_id) => {components[self.id].plans.iter_mut().enumerate().find(|(i,plan)| {plan.plan_action_id == action_id}).and_then(|(i, plan)| {Some(i)}).and_then(|i| {components[self.id].plans.remove(i); Some(0)});},
        }
    }
}

impl<ID:Identify> Component<ID> for Planner {
    type CE = PlannerEvent<ID>;
    type SC = StaticPlanner;
    fn from_static(static_comp:&Self::SC) -> Self {
        Self { finished_actions: Vec::with_capacity(4), plans: Vec::with_capacity(4) }
    }
}