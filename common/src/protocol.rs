use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentMessage {
    #[serde(rename = "request")]
    Request {
        id: String,
        method: String,
        params: serde_json::Value,
    },
    #[serde(rename = "response")]
    Response {
        id: String,
        result: Option<serde_json::Value>,
        error: Option<String>,
    },
    #[serde(rename = "event")]
    Event {
        event_type: String,
        data: serde_json::Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- Request roundtrip ---

    #[test]
    fn request_serialization_roundtrip() {
        let msg = AgentMessage::Request {
            id: "req-001".to_string(),
            method: "deploy".to_string(),
            params: json!({"app_id": "abc123"}),
        };
        let json_str = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            AgentMessage::Request { id, method, params } => {
                assert_eq!(id, "req-001");
                assert_eq!(method, "deploy");
                assert_eq!(params["app_id"], "abc123");
            }
            _ => panic!("Expected Request variant"),
        }
    }

    // --- Response roundtrip ---

    #[test]
    fn response_with_result_roundtrip() {
        let msg = AgentMessage::Response {
            id: "res-001".to_string(),
            result: Some(json!({"status": "ok"})),
            error: None,
        };
        let json_str = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            AgentMessage::Response { id, result, error } => {
                assert_eq!(id, "res-001");
                assert_eq!(result.unwrap()["status"], "ok");
                assert!(error.is_none());
            }
            _ => panic!("Expected Response variant"),
        }
    }

    #[test]
    fn response_with_error_roundtrip() {
        let msg = AgentMessage::Response {
            id: "res-002".to_string(),
            result: None,
            error: Some("deploy failed".to_string()),
        };
        let json_str = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            AgentMessage::Response { id, result, error } => {
                assert_eq!(id, "res-002");
                assert!(result.is_none());
                assert_eq!(error.unwrap(), "deploy failed");
            }
            _ => panic!("Expected Response variant"),
        }
    }

    // --- Event roundtrip ---

    #[test]
    fn event_serialization_roundtrip() {
        let msg = AgentMessage::Event {
            event_type: "deploy.started".to_string(),
            data: json!({"app_id": "xyz", "deploy_id": "d-001"}),
        };
        let json_str = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            AgentMessage::Event { event_type, data } => {
                assert_eq!(event_type, "deploy.started");
                assert_eq!(data["app_id"], "xyz");
                assert_eq!(data["deploy_id"], "d-001");
            }
            _ => panic!("Expected Event variant"),
        }
    }

    // --- JSON shape tests ---

    #[test]
    fn request_json_has_type_tag() {
        let msg = AgentMessage::Request {
            id: "r1".to_string(),
            method: "status".to_string(),
            params: json!({}),
        };
        let value: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(value["type"], "request");
        assert_eq!(value["id"], "r1");
        assert_eq!(value["method"], "status");
    }

    #[test]
    fn response_json_has_type_tag() {
        let msg = AgentMessage::Response {
            id: "r2".to_string(),
            result: Some(json!("done")),
            error: None,
        };
        let value: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(value["type"], "response");
        assert_eq!(value["id"], "r2");
        assert_eq!(value["result"], "done");
        assert!(value["error"].is_null());
    }

    #[test]
    fn event_json_has_type_tag() {
        let msg = AgentMessage::Event {
            event_type: "build.complete".to_string(),
            data: json!(null),
        };
        let value: serde_json::Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(value["type"], "event");
        assert_eq!(value["event_type"], "build.complete");
    }

    // --- Deserialization from raw JSON ---

    #[test]
    fn deserialize_request_from_raw_json() {
        let json = r#"{"type":"request","id":"x","method":"ping","params":null}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        match msg {
            AgentMessage::Request { id, method, params } => {
                assert_eq!(id, "x");
                assert_eq!(method, "ping");
                assert!(params.is_null());
            }
            _ => panic!("Expected Request"),
        }
    }

    #[test]
    fn deserialize_response_from_raw_json() {
        let json = r#"{"type":"response","id":"y","result":42,"error":null}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        match msg {
            AgentMessage::Response { id, result, error } => {
                assert_eq!(id, "y");
                assert_eq!(result.unwrap(), json!(42));
                assert!(error.is_none());
            }
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn deserialize_event_from_raw_json() {
        let json = r#"{"type":"event","event_type":"log","data":{"line":"hello"}}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        match msg {
            AgentMessage::Event { event_type, data } => {
                assert_eq!(event_type, "log");
                assert_eq!(data["line"], "hello");
            }
            _ => panic!("Expected Event"),
        }
    }

    #[test]
    fn invalid_type_tag_fails_deserialization() {
        let json = r#"{"type":"unknown","id":"z"}"#;
        let result = serde_json::from_str::<AgentMessage>(json);
        assert!(result.is_err());
    }
}
