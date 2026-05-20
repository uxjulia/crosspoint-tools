use serde::{Deserialize, Serialize};
use unlocker_core::types::ArmServerSpec;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Ping,
    IsEnable {
        ssid: String,
        psk: String,
    },
    IsDisable,
    PfctlAdd {
        from_port: u16,
        to_port: u16,
    },
    PfctlRemove,
    DhcpdRead,
    BridgeIp,
    FullCleanup,

    // ── Spoofing servers (DNS + HTTP + HTTPS) ──
    /// Start the manifest/DNS/cert servers bound to the bridge IP.
    ArmServers(ArmServerSpec),
    /// Block until the device's first manifest request hits the server.
    WaitManifest,
    /// Block until the firmware binary download completes.
    WaitFirmware,
    /// Stop the spoofing servers.
    DisarmServers,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Ok { data: serde_json::Value },
    Err { error: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DhcpLease {
    pub ip: String,
    pub mac: String,
    pub name: Option<String>,
}
