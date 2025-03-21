mod bindings;

use crate::bindings::exports::ntwk::theater::actor::Guest;
use crate::bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClient;
use crate::bindings::ntwk::theater::message_server_host::request;
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::store;
use crate::bindings::ntwk::theater::supervisor::{spawn, stop_child};
use crate::bindings::ntwk::theater::types::State;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
struct InitData {
    build_store_id: Option<String>,
    runtime_content_fs_actor_id: String,
    anthropic_api_key: String,
}

#[derive(Serialize, Deserialize)]
struct AppState {
    child_id: Option<String>,
    programmer_actor_id: String,
    runtime_content_fs_actor_id: String,
    build_store_id: String,
}

/// Structure to hold build result information
#[derive(Debug, Serialize, Deserialize, Clone)]
struct BuildOutput {
    success: bool,
    stdout: String,
    stderr: String,
    wasm_path: Option<String>,
    wasm_hash: Option<String>,
    build_logs: Vec<String>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum Action {
    Start,
    Stop,
    Build,
    Change(String),
}

/// Result of a get info operation
#[derive(Serialize, Deserialize, Debug)]
pub struct InfoResult {
    pub head_hash: String,
    pub store_id: String,
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

        let build_store_id = match init_state.build_store_id {
            Some(id) => id,
            None => store::new().map_err(|e| e.to_string())?,
        };

        log(&format!("Build store ID: {}", build_store_id));

        let programmer_init = json!({
            "content_fs_actor_id": init_state.runtime_content_fs_actor_id,
            "anthropic_api_key": init_state.anthropic_api_key,
        });

        let programmer_actor_id = spawn(
            "/Users/colinrozzi/work/actors/programmer/manifest.toml",
            Some(&serde_json::to_vec(&programmer_init).unwrap()),
        )
        .expect("Failed to spawn programmer actor");

        log(&format!("Programmer actor ID: {}", programmer_actor_id));

        let app_state = AppState {
            child_id: None,
            runtime_content_fs_actor_id: init_state.runtime_content_fs_actor_id,
            build_store_id,
            programmer_actor_id,
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

        let child_manifest = r#"
name = "child"
version = "0.1.0"
description = "An HTTP server Theater actor"
component_path = "store://44768743-9232-43de-9819-47c210588b2b/wasm"

[interface]
implements = "ntwk:theater/actor"
requires = []

[[handlers]]
type = "runtime"
config = {}

[[handlers]]
type = "http-framework"
config = {}
        "#;

        let action: Action = serde_json::from_slice(&data).map_err(|e| e.to_string())?;

        let response = match action {
            Action::Start => {
                let child_id = spawn(child_manifest, None).map_err(|e| e.to_string())?;
                log(&format!("Spawned child actor with ID: {}", child_id));
                app_state.child_id = Some(child_id);
                "Started child actor".as_bytes().to_vec()
            }
            Action::Stop => {
                if let Some(child_id) = app_state.child_id.take() {
                    log(&format!("Stopping child actor with ID: {}", child_id));
                    stop_child(&child_id).map_err(|e| e.to_string())?;
                    app_state.child_id = None;
                    "Stopped child actor".as_bytes().to_vec()
                } else {
                    "No child actor to stop".as_bytes().to_vec()
                }
            }
            Action::Build => {
                let runtime_info_response = request(
                    &app_state.runtime_content_fs_actor_id,
                    &serde_json::to_vec(&json!({"action": "get-info", "params": []})).unwrap(),
                )
                .expect("Failed to get programmer actor info");

                log(&format!(
                    "Received runtime info: {}",
                    String::from_utf8(runtime_info_response.clone()).unwrap()
                ));

                let build_actor_id =
                    spawn("/Users/colinrozzi/work/actors/build-actor/actor.toml", None)
                        .expect("Failed to spawn build actor");
                log(&format!("Build actor ID: {}", build_actor_id.clone()));

                let runtime_info_value: Value =
                    serde_json::from_slice::<Value>(&runtime_info_response)
                        .expect("Failed to parse runtime info");

                log(&format!("Runtime info value: {:?}", runtime_info_value));

                let runtime_info = runtime_info_value.get("data").unwrap();

                log(&format!("Runtime info: {:?}", runtime_info));

                let cur_info = serde_json::from_value::<InfoResult>(runtime_info.clone())
                    .expect("Failed to parse programmer actor info");
                log(&format!("Programmer actor response: {:?}", cur_info));

                let build_state = json!({
                    "fs_hash": cur_info.head_hash,
                    "store_id": cur_info.store_id,
                    "build_store_id": app_state.build_store_id,
                });
                let result = request(&build_actor_id, &serde_json::to_vec(&build_state).unwrap())
                    .map_err(|e| e.to_string())?;
                log(&format!(
                    "Build actor response: {}",
                    String::from_utf8(result.clone()).unwrap()
                ));

                let result: BuildOutput =
                    serde_json::from_slice(&result).map_err(|e| e.to_string())?;
                log(&format!("Build output: {:?}", result));

                let bytes = store::get_by_label(&app_state.build_store_id, "wasm")
                    .map_err(|e| e.to_string())?;

                log(&format!("Wasm bytes: {:?}", bytes));

                stop_child(&build_actor_id).map_err(|e| e.to_string())?;

                "Built".as_bytes().to_vec()
            }
            Action::Change(req) => {
                log(&format!("Received change request: {}", req));

                let result = request(
                    &app_state.programmer_actor_id,
                    &serde_json::to_vec(&json!({"change": req})).unwrap(),
                )
                .expect("Failed to send change request");

                log(&format!(
                    "Received programmer actor response: {}",
                    String::from_utf8(result.clone()).unwrap()
                ));

                "Changed".as_bytes().to_vec()
            }
        };

        // Save the updated state
        let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        let updated_state = Some(updated_state_bytes);

        Ok((updated_state, (response,)))
    }

    fn handle_channel_open(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (bindings::exports::ntwk::theater::message_server_client::Json,),
    ) -> Result<
        (
            Option<bindings::exports::ntwk::theater::message_server_client::Json>,
            (bindings::exports::ntwk::theater::message_server_client::ChannelAccept,),
        ),
        String,
    > {
        Ok((
            state,
            (
                bindings::exports::ntwk::theater::message_server_client::ChannelAccept {
                    accepted: true,
                    message: None,
                },
            ),
        ))
    }

    fn handle_channel_close(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (String,),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String>
    {
        Ok((state,))
    }

    fn handle_channel_message(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (
            String,
            bindings::exports::ntwk::theater::message_server_client::Json,
        ),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String>
    {
        log("runtime-content-fs: Received channel message");
        Ok((state,))
    }
}

bindings::export!(Actor with_types_in bindings);
