mod bindings;

use crate::bindings::exports::ntwk::theater::actor::Guest;
use crate::bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClient;
use crate::bindings::ntwk::theater::message_server_host::request;
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::supervisor::{spawn, stop_child};
use crate::bindings::ntwk::theater::types::State;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
struct InitData {
    fs_hash: String,
    store_id: String,
}

#[derive(Serialize, Deserialize)]
struct AppState {
    child_id: Option<String>,
    build_actor_id: String,
    fs_hash: String,
    store_id: String,
}

struct Actor;
impl Guest for Actor {
    fn init(state: State, params: (String,)) -> Result<(State,), String> {
        log("Initializing manager actor");
        let (param,) = params;
        log(&format!("Init parameter: {}", param));
        log(&format!("State: {:?}", state));

        let init_state =
            serde_json::from_slice::<InitData>(&state.unwrap()).map_err(|e| e.to_string())?;

        let build_actor_id = spawn("/Users/colinrozzi/work/actors/build-actor/actor.toml", None)
            .expect("Failed to spawn build actor");

        log(&format!("Build actor ID: {}", build_actor_id));

        let app_state = AppState {
            child_id: None,
            build_actor_id,
            fs_hash: init_state.fs_hash,
            store_id: init_state.store_id,
        };
        let state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;

        // Create the initial state
        let new_state = Some(state_bytes);

        Ok((new_state,))
    }
}

impl MessageServerClient for Actor {
    fn handle_send(
        state: Option<Vec<u8>>,
        params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling send message");
        let (data,) = params;

        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let app_state: AppState = if !state_bytes.is_empty() {
            serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?
        } else {
            Err("No state found".to_string())?
        };

        // Try to parse the message as a string
        if let Ok(message) = String::from_utf8(data) {
            log(&format!("Received message: {}", message));
        } else {
        }

        // Save the updated state
        let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        let updated_state = Some(updated_state_bytes);

        Ok((updated_state,))
    }

    fn handle_request(
        state: Option<Vec<u8>>,
        params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>, (Vec<u8>,)), String> {
        log("Handling request message");
        let (data,) = params;

        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let mut app_state: AppState = if !state_bytes.is_empty() {
            serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?
        } else {
            Err("No state found".to_string())?
        };

        // Try to parse the message as a string
        let response = if let Ok(message) = String::from_utf8(data.clone()) {
            log(&format!("Received request: {}", message));

            match message.as_str() {
                "start" => {
                    let child_id = spawn("/Users/colinrozzi/work/actors/child/manifest.toml", None)
                        .map_err(|e| e.to_string())?;
                    log(&format!("Spawned child actor with ID: {}", child_id));
                    app_state.child_id = Some(child_id);
                    "Started child actor".as_bytes().to_vec()
                }
                "stop" => {
                    if let Some(child_id) = app_state.child_id.take() {
                        log(&format!("Stopping child actor with ID: {}", child_id));
                        stop_child(&child_id).map_err(|e| e.to_string())?;
                        app_state.child_id = None;
                        "Stopped child actor".as_bytes().to_vec()
                    } else {
                        "No child actor to stop".as_bytes().to_vec()
                    }
                }
                "build" => {
                    let build_state = json!({
                        "fs_hash": app_state.fs_hash,
                        "store_id": app_state.store_id,
                    });
                    let result = request(
                        &app_state.build_actor_id,
                        &serde_json::to_vec(&build_state).unwrap(),
                    )
                    .map_err(|e| e.to_string())?;
                    log(&format!(
                        "Build actor response: {}",
                        String::from_utf8(result.clone()).unwrap()
                    ));
                    result
                }
                _ => "Unknown request".as_bytes().to_vec(),
            }
        } else {
            log("Received binary data request");
            // Just echo back the data
            data
        };

        // Save the updated state
        let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        let updated_state = Some(updated_state_bytes);

        Ok((updated_state, (response,)))
    }
}

bindings::export!(Actor with_types_in bindings);
