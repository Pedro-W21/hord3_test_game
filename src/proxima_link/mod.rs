use std::{sync::mpmc::{self, Receiver, Sender}, thread, time::Duration};

use hord3::horde::game_engine::world::WorldComputeHandler;
use proxima_backend::{ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant}, web_payloads::{AIPayload, AIResponse, AuthPayload, AuthResponse}};
use serde::de::DeserializeOwned;
use to_from_bytes_derive::{FromBytes, ToBytes};

use crate::{game_engine::{CoolGameEngineTID, CoolVoxel}, game_entity::{director::{DirectorEvent, DirectorUpdate}, GameEntityVecRead}, game_map::GameMap};

pub struct ProximaLink {
    auth_key:String,
    device_id:usize,
    server_url:String,
    request_receiver:Receiver<HordeProximaAIRequest>,
    response_sender:Sender<HordeProximaAIResponse>
}

impl ProximaLink {
    pub fn initialize(username:String, password:String, url:String) -> Result<(Sender<HordeProximaAIRequest>, Receiver<HordeProximaAIResponse>), ()> {
        let auth = AuthPayload::new(password, username);
        let response = send_payload::<AuthPayload, AuthResponse>(auth, format!("{}/auth", url.clone()));
        match response {
            Ok(auth_response) => {
                let (request_sender, request_receiver) = mpmc::channel();
                let (response_sender, response_receiver) = mpmc::channel();
                let link = Self {
                    auth_key:auth_response.session_token,
                    device_id:auth_response.device_id,
                    server_url:url,
                    request_receiver,
                    response_sender
                };
                println!("Starting thread");
                thread::spawn(move || {
                    link.link_loop();
                });
                Ok((   
                    request_sender,
                    response_receiver
                ))
            },
            Err(_) => Err(()) 
        }
    }
    fn link_loop(&self) {
        println!("Starting link loop");
        loop {
            match self.request_receiver.recv() {
                Ok(request) => {
                    self.handle_request(request);
                },
                Err(error) => {
                    dbg!(error);
                    break;
                }
            }
        }
    }
    fn handle_request(&self, request:HordeProximaAIRequest) {
        let response_sender = self.response_sender.clone();
        let url = self.server_url.clone();
        let key = self.auth_key.clone();
        thread::spawn(move || {
            let result = send_payload::<AIPayload, AIResponse>(AIPayload::new(key, request.request), format!("{}/ai", url));
            
            match result {
                Ok(response) => {
                    match response.reply {
                        EndpointResponseVariant::Block(part) => {
                            response_sender.send(HordeProximaAIResponse { request_id: request.request_id, entity_id: request.entity_id, response:Some(part.data_to_text().concat()) });
                        },
                        _ => {response_sender.send(HordeProximaAIResponse { request_id: request.request_id, entity_id: request.entity_id, response:None });},
                    }
                },
                Err(()) => {response_sender.send(HordeProximaAIResponse { request_id: request.request_id, entity_id: request.entity_id, response:None });},
            }
        });
    }
}

pub struct HordeProximaAIRequest {
    request_id:usize,
    entity_id:CoolGameEngineTID,
    request:EndpointRequestVariant
}

impl HordeProximaAIRequest {
    pub fn new(
        request_id:usize,
        entity_id:CoolGameEngineTID,
        request:EndpointRequestVariant
    ) -> Self {
        Self { request_id, entity_id, request }
    }
}

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct HordeProximaAIResponse {
    pub request_id:usize,
    entity_id:CoolGameEngineTID,
    pub response:Option<String>
}

impl HordeProximaAIResponse {
    pub fn apply<'a>(
        self,
        first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>,
        world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>,
    ) {
        match self.entity_id {
            CoolGameEngineTID::entity_1(id) => {
                first_ent.tunnels.director_out.send(DirectorEvent::new(id, None, DirectorUpdate::LLMAddToResponses(self.clone())));
            },
            CoolGameEngineTID::entity_2(id) => {
                second_ent.tunnels.director_out.send(DirectorEvent::new(id, None, DirectorUpdate::LLMAddToResponses(self.clone())));
            },
            _ => ()
        }
    }
}

fn send_payload<T:serde::ser::Serialize, U:DeserializeOwned>(payload:T, url:String) -> Result<U, ()> {
    let response = reqwest::blocking::Client::new()
        .post(url)
        .json(&payload)
        .timeout(Duration::from_millis(500000))
        .send();
    //println!("Received response");
    match response {
        Ok(data) => {
            println!("Response is okayyy");
            if data.status().is_success() {
                println!("Response got JSON");
                match data.json() {
                    Ok(data) => {
                        println!("GOT GOOD DATA");
                        Ok(data)
                    },
                    Err(error) => {dbg!(error);Err(())}
                }
            }
            else {
                Err(())
            }
            
        },
        Err(error) => {dbg!(error);Err(())}
    }
}