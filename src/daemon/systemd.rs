use crate::config::IcefallConfig;

pub fn generate_socket_unit(config: &IcefallConfig) -> String {
    let port = config.listen_port;

    format!(
        r#"[Unit]
Description=Icefall Socket

[Socket]
ListenStream=0.0.0.0:{port}
FileDescriptorStoreMax=1
ReusePort=true

[Install]
WantedBy=sockets.target
"#
    )
}

pub fn generate_service_unit(_config: &IcefallConfig) -> String {
    let config_path =
        std::env::var("ICEFALL_CONFIG").unwrap_or_else(|_| "/etc/icefall/config.toml".to_string());

    format!(
        r#"[Unit]
Description=Icefall Deployment Platform
After=network.target docker.service
Requires=docker.service icefall.socket
After=icefall.socket

[Service]
Type=notify
ExecStart=/usr/local/bin/icefall daemon start
ExecStopPost=-/var/lib/icefall/updates/icefall.rollback rollback --check
Restart=on-failure
RestartSec=2
StartLimitBurst=3
StartLimitIntervalSec=300
WatchdogSec=60
KillMode=mixed
TimeoutStopSec=30
Environment=ICEFALL_CONFIG={config_path}

[Install]
WantedBy=multi-user.target
"#
    )
}

pub fn is_systemd_managed() -> bool {
    std::env::var("INVOCATION_ID").is_ok()
}

pub fn notify_ready() {
    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]);
}

pub fn notify_watchdog() {
    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
}

pub fn notify_stopping() {
    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Stopping]);
}
