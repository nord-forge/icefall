use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::db::models::now_iso8601;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcefallEvent {
    pub id: u64,
    pub timestamp: String,
    pub app_id: Option<String>,
    pub deploy_id: Option<String>,
    pub event_type: EventType,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    BuildStepStart,
    BuildStepOutput,
    BuildStepComplete,
    BuildComplete,
    DeployStatus,
    HealthStatus,
    UpdateAvailable,
    UpdateScheduled,
    UpdateStep,
    ServerConnected,
    ServerDisconnected,
    DiskAlert,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BuildStepStart => "build.step.start",
            Self::BuildStepOutput => "build.step.output",
            Self::BuildStepComplete => "build.step.complete",
            Self::BuildComplete => "build.complete",
            Self::DeployStatus => "deploy.status",
            Self::HealthStatus => "health.status",
            Self::UpdateAvailable => "update.available",
            Self::UpdateScheduled => "update.scheduled",
            Self::UpdateStep => "update.step",
            Self::ServerConnected => "server.connected",
            Self::ServerDisconnected => "server.disconnected",
            Self::DiskAlert => "server.disk.alert",
        }
    }
}

pub struct EventBus {
    sender: broadcast::Sender<IcefallEvent>,
    next_id: AtomicU64,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            next_id: AtomicU64::new(1),
        }
    }

    pub fn emit(
        &self,
        event_type: EventType,
        app_id: Option<&str>,
        deploy_id: Option<&str>,
        data: serde_json::Value,
    ) {
        let event = IcefallEvent {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            timestamp: now_iso8601(),
            app_id: app_id.map(std::string::ToString::to_string),
            deploy_id: deploy_id.map(std::string::ToString::to_string),
            event_type,
            data,
        };
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<IcefallEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn emit_and_subscribe() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit(
            EventType::BuildComplete,
            Some("app-1"),
            Some("deploy-1"),
            serde_json::json!({"status": "success"}),
        );

        let event = rx.recv().await.unwrap();
        assert_eq!(event.event_type, EventType::BuildComplete);
        assert_eq!(event.app_id, Some("app-1".to_string()));
        assert_eq!(event.id, 1);
    }

    #[tokio::test]
    async fn monotonic_ids() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit(EventType::BuildStepStart, None, None, serde_json::json!({}));
        bus.emit(
            EventType::BuildStepComplete,
            None,
            None,
            serde_json::json!({}),
        );

        let e1 = rx.recv().await.unwrap();
        let e2 = rx.recv().await.unwrap();
        assert!(e2.id > e1.id);
    }

    #[tokio::test]
    async fn multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.emit(
            EventType::DeployStatus,
            None,
            None,
            serde_json::json!({"status": "running"}),
        );

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.id, e2.id);
    }

    #[tokio::test]
    async fn lagged_subscriber_recovers() {
        let bus = EventBus::new(4);
        let mut rx = bus.subscribe();

        for i in 0..10 {
            bus.emit(
                EventType::BuildStepOutput,
                None,
                None,
                serde_json::json!({"line": i}),
            );
        }

        match rx.recv().await {
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            other => panic!("expected Lagged, got {other:?}"),
        }

        let event = rx.recv().await.unwrap();
        assert!(event.id > 0);
    }

    #[test]
    fn event_type_as_str() {
        assert_eq!(EventType::BuildStepStart.as_str(), "build.step.start");
        assert_eq!(EventType::DeployStatus.as_str(), "deploy.status");
    }
}
