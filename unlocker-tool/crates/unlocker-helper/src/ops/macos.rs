//! Privileged operations. Pragmatic shell-outs to system tools.
//!
//! Every state-changing op records its intent in the state file *before*
//! acting, so a crash mid-op is recoverable on next launch.

use crate::proto::DhcpLease;
use crate::state;
use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;
use tokio::process::Command;

const NAT_PLIST: &str = "/Library/Preferences/SystemConfiguration/com.apple.nat.plist";
const NAT_PLIST_BACKUP: &str = "/var/db/com.sofriendly.crosspoint.unlocker.nat.plist.bak";
const PF_RULES_PATH: &str = "/var/db/com.sofriendly.crosspoint.unlocker.pf.conf";
const PREFS_PLIST: &str = "/Library/Preferences/SystemConfiguration/preferences.plist";
const PREFS_PLIST_BACKUP: &str = "/var/db/com.sofriendly.crosspoint.unlocker.preferences.plist.bak";

async fn sh(prog: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(prog)
        .args(args)
        .output()
        .await
        .with_context(|| format!("spawn {prog} {args:?}"))?;
    if !out.status.success() {
        bail!(
            "{prog} {args:?} failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

// ── Internet Sharing ─────────────────────────────────────────────────────────

const ADHOC_SERVICE_NAME: &str = "Xteink Unlocker";
const ADHOC_IP: &str = "10.10.10.1";
const LOOPBACK_IP: &str = "127.0.0.1";
const LOOPBACK_NETMASK: &str = "0xff000000";

async fn restore_loopback() {
    let _ = sh(
        "ifconfig",
        &[
            "lo0",
            "inet",
            LOOPBACK_IP,
            "netmask",
            LOOPBACK_NETMASK,
            "up",
        ],
    )
    .await;
    let _ = sh("ifconfig", &["lo0", "-alias", ADHOC_IP]).await;
}

/// Create a fake network service on lo0 so Internet Sharing sees an
/// "active" upstream even though there's no real internet connection.
async fn create_adhoc_upstream() -> Result<()> {
    // Remove stale service if it exists from a prior run.
    let _ = sh(
        "networksetup",
        &["-removenetworkservice", ADHOC_SERVICE_NAME],
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Create may fail if it already exists (race or incomplete cleanup).
    match sh(
        "networksetup",
        &["-createnetworkservice", ADHOC_SERVICE_NAME, "lo0"],
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(?e, "createnetworkservice failed, may already exist");
        }
    }
    // Set the IP regardless — works even if the service already existed.
    sh(
        "networksetup",
        &[
            "-setmanual",
            ADHOC_SERVICE_NAME,
            ADHOC_IP,
            "255.255.255.255",
        ],
    )
    .await?;
    tracing::info!("adhoc upstream service ready on lo0");
    Ok(())
}

async fn remove_adhoc_upstream() {
    let _ = sh(
        "networksetup",
        &["-removenetworkservice", ADHOC_SERVICE_NAME],
    )
    .await;
    // networksetup refuses to remove the last service on lo0 ("you cannot
    // remove ... because there aren't any other network services on
    // Loopback"), leaving an orphaned entry in preferences.plist that the
    // GUI can't delete either (System Settings crashes). Surgically remove
    // any leftover entry from the plist directly.
    if let Err(e) = purge_adhoc_from_prefs_plist().await {
        tracing::warn!(?e, "failed to purge adhoc service from preferences.plist");
    }
    restore_loopback().await;
    tracing::info!("removed adhoc upstream service");
}

/// Remove orphaned "Xteink Unlocker" entries from
/// /Library/Preferences/SystemConfiguration/preferences.plist when
/// `networksetup -removenetworkservice` couldn't.
async fn purge_adhoc_from_prefs_plist() -> Result<()> {
    use plist::Value;

    if !Path::new(PREFS_PLIST).exists() {
        return Ok(());
    }

    let bytes = tokio::fs::read(PREFS_PLIST).await?;
    let mut root: Value = plist::from_bytes(&bytes).context("parse preferences.plist")?;

    let root_dict = root
        .as_dictionary_mut()
        .ok_or_else(|| anyhow!("preferences.plist root is not a dictionary"))?;

    // Find UUIDs of services whose UserDefinedName matches ours.
    let mut victim_uuids: Vec<String> = Vec::new();
    if let Some(services) = root_dict
        .get("NetworkServices")
        .and_then(|v| v.as_dictionary())
    {
        for (uuid, svc) in services {
            if let Some(name) = svc
                .as_dictionary()
                .and_then(|d| d.get("UserDefinedName"))
                .and_then(|v| v.as_string())
            {
                if name == ADHOC_SERVICE_NAME {
                    victim_uuids.push(uuid.clone());
                }
            }
        }
    }

    if victim_uuids.is_empty() {
        return Ok(());
    }

    // Back up once before mutating.
    if !Path::new(PREFS_PLIST_BACKUP).exists() {
        tokio::fs::copy(PREFS_PLIST, PREFS_PLIST_BACKUP).await.ok();
    }

    // Remove from NetworkServices.
    if let Some(services) = root_dict
        .get_mut("NetworkServices")
        .and_then(|v| v.as_dictionary_mut())
    {
        for uuid in &victim_uuids {
            services.remove(uuid);
        }
    }

    // Remove references from each Set's Network.Service dict and ServiceOrder.
    if let Some(sets) = root_dict
        .get_mut("Sets")
        .and_then(|v| v.as_dictionary_mut())
    {
        for (_set_uuid, set_val) in sets.iter_mut() {
            let Some(set_dict) = set_val.as_dictionary_mut() else {
                continue;
            };
            let Some(network) = set_dict
                .get_mut("Network")
                .and_then(|v| v.as_dictionary_mut())
            else {
                continue;
            };

            if let Some(svc_dict) = network
                .get_mut("Service")
                .and_then(|v| v.as_dictionary_mut())
            {
                for uuid in &victim_uuids {
                    svc_dict.remove(uuid);
                }
            }

            if let Some(global) = network
                .get_mut("Global")
                .and_then(|v| v.as_dictionary_mut())
            {
                if let Some(order) = global
                    .get_mut("ServiceOrder")
                    .and_then(|v| v.as_array_mut())
                {
                    order.retain(|item| {
                        item.as_string()
                            .map(|s| !victim_uuids.iter().any(|u| u == s))
                            .unwrap_or(true)
                    });
                }
            }
        }
    }

    let mut buf = Vec::new();
    plist::to_writer_xml(&mut buf, &root).context("serialize preferences.plist")?;
    tokio::fs::write(PREFS_PLIST, buf).await?;

    tracing::info!(
        count = victim_uuids.len(),
        "purged orphaned Xteink Unlocker entries from preferences.plist"
    );
    Ok(())
}

async fn wifi_device() -> Result<String> {
    let out = sh("networksetup", &["-listallhardwareports"]).await?;
    let mut in_wifi_block = false;

    for line in out.lines() {
        let line = line.trim();
        if let Some(port) = line.strip_prefix("Hardware Port: ") {
            in_wifi_block = port == "Wi-Fi" || port == "AirPort";
            continue;
        }

        if in_wifi_block {
            if let Some(device) = line.strip_prefix("Device: ") {
                let device = device.trim();
                if !device.is_empty() {
                    return Ok(device.to_string());
                }
            }
        }
    }

    Err(anyhow!("could not find Wi-Fi hardware device"))
}

pub async fn is_enable(ssid: &str, psk: &str) -> Result<()> {
    // Back up the existing plist if we haven't already.
    if Path::new(NAT_PLIST).exists() && !Path::new(NAT_PLIST_BACKUP).exists() {
        tokio::fs::copy(NAT_PLIST, NAT_PLIST_BACKUP).await.ok();
    }

    tracing::info!("stopping existing NetworkSharing");
    let _ = sh("launchctl", &["bootout", "system/com.apple.NetworkSharing"]).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Internet Sharing requires an active network service with an IP to act
    // as the upstream. Create a fake one on lo0 so it works even when the
    // Mac has no wired connection.
    create_adhoc_upstream().await?;

    // Disconnect Wi-Fi from the current network. Internet Sharing needs to
    // reconfigure Wi-Fi from client mode to AP mode — it can't do that while
    // associated with a network. We leave the radio on.
    let wifi = wifi_device().await?;
    tracing::info!(%wifi, "disconnecting Wi-Fi from current network");
    let _ = sh("networksetup", &["-setairportpower", &wifi, "off"]).await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let _ = sh("networksetup", &["-setairportpower", &wifi, "on"]).await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Write NAT plist so Internet Sharing is pre-configured when the user
    // enables it in System Settings.
    tracing::info!(%ssid, %wifi, "writing NAT plist with lo0 upstream");
    write_nat_plist("lo0", &wifi, ssid, psk).await?;
    state::mutate(|s| s.internet_sharing_active = true).await?;

    tracing::info!(%ssid, "Internet Sharing configured — user must enable in System Settings");
    Ok(())
}

pub async fn is_disable() -> Result<()> {
    let _ = sh("launchctl", &["bootout", "system/com.apple.NetworkSharing"]).await;

    // Restore the prior plist if we backed one up.
    if Path::new(NAT_PLIST_BACKUP).exists() {
        tokio::fs::rename(NAT_PLIST_BACKUP, NAT_PLIST).await.ok();
    } else {
        tokio::fs::remove_file(NAT_PLIST).await.ok();
    }

    // Remove the fake upstream service.
    remove_adhoc_upstream().await;

    state::mutate(|s| s.internet_sharing_active = false).await?;
    tracing::info!("Internet Sharing disabled");
    Ok(())
}

async fn write_nat_plist(upstream: &str, wifi_device: &str, ssid: &str, psk: &str) -> Result<()> {
    use plist::Value;
    let mut airport = plist::Dictionary::new();
    airport.insert("40BitEncrypt".into(), Value::Integer(0i64.into()));
    airport.insert("Channel".into(), Value::Integer(11i64.into()));
    airport.insert("Enabled".into(), Value::Integer(1i64.into()));
    airport.insert("NetworkName".into(), Value::String(ssid.to_string()));
    airport.insert(
        "NetworkPassword".into(),
        Value::Data(psk.as_bytes().to_vec()),
    );

    let mut nat = plist::Dictionary::new();
    nat.insert("Enabled".into(), Value::Integer(1i64.into()));
    nat.insert(
        "SharingDevices".into(),
        Value::Array(vec![Value::String(wifi_device.to_string())]),
    );
    nat.insert(
        "PrimaryInterface".into(),
        Value::Dictionary({
            let mut p = plist::Dictionary::new();
            p.insert("Device".into(), Value::String(upstream.to_string()));
            p.insert("Enabled".into(), Value::Integer(1i64.into()));
            p
        }),
    );
    nat.insert("AirPort".into(), Value::Dictionary(airport));

    let mut root = plist::Dictionary::new();
    root.insert("NAT".into(), Value::Dictionary(nat));

    let bytes = {
        let mut buf = Vec::new();
        plist::to_writer_xml(&mut buf, &Value::Dictionary(root))?;
        buf
    };
    if let Some(parent) = Path::new(NAT_PLIST).parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(NAT_PLIST, bytes).await?;
    Ok(())
}

// ── pfctl (DNS port redirect) ────────────────────────────────────────────────

pub async fn pfctl_add(from_port: u16, to_port: u16) -> Result<()> {
    let bridge = bridge_ip()
        .await
        .unwrap_or_else(|_| "192.168.2.1".to_string());

    // Use a single pf config that includes our rdr rules inline (not via anchors).
    // Anchors require being referenced in the main /etc/pf.conf which we don't
    // want to modify. Instead, load a standalone ruleset with -a.
    //
    // Actually, the simplest approach: use the nat-anchor that Internet Sharing
    // already sets up ("com.apple.internet-sharing"), or load rdr rules directly
    // via a transient anchor using echo | pfctl.
    //
    // The explicit ICMP pass for type 3 code 4 (fragmentation-needed) keeps
    // Path-MTU Discovery working through our redirect, so big firmware
    // transfers can recover if the device guesses too high.
    // pf requires rules in a fixed order: options, normalization, queueing,
    // translation (rdr/nat), then filtering (pass/block). The rdr rules must
    // come before the icmp pass.
    let rules = format!(
        "rdr pass on bridge100 inet proto udp from any to any port {from} -> {bridge} port {to}\n\
         rdr pass on bridge100 inet proto tcp from any to any port {from} -> {bridge} port {to}\n\
         pass on bridge100 inet proto icmp icmp-type 3 code 4\n",
        from = from_port,
        to = to_port,
        bridge = bridge,
    );
    tokio::fs::write(PF_RULES_PATH, &rules).await?;
    state::mutate(|s| s.pfctl_anchor_loaded = true).await?;

    // Load into the Internet Sharing anchor which is already referenced
    // in the main ruleset — this piggybacks on Apple's existing anchor point.
    sh(
        "pfctl",
        &["-a", "com.apple.internet-sharing", "-f", PF_RULES_PATH],
    )
    .await?;

    // Enable pf if not already enabled.
    let _ = sh("pfctl", &["-E"]).await;

    // Flush the global state table. Without this, leftover TCP flow entries
    // from a previous run (cancelled mid-OTA, force-quit, etc.) can match
    // the device's new connections and silently misroute them — a known
    // cause of "first attempt fails, reboot fixes it" reports.
    let _ = sh("pfctl", &["-F", "states"]).await;

    // Pin bridge100 MTU to 1500. Our lo0 fake upstream has MTU 16384, which
    // can poison PMTU discovery on the return path: the manifest fits in one
    // packet (works fine) but the multi-MB firmware download gets blackholed
    // by oversize segments. 1500 matches the device's Wi-Fi link.
    let _ = sh("ifconfig", &["bridge100", "mtu", "1500"]).await;

    // Disable TCP Segmentation Offload + Large Receive Offload on bridge100
    // and the underlying Wi-Fi NIC. Apple Silicon Wi-Fi drivers hand the NIC
    // 64KB super-segments that the bridge forward path mis-resegments,
    // killing large transfers (firmware) while small ones (manifest) get
    // through. Intel Macs don't show the bug. We re-enable on teardown.
    disable_offload("bridge100").await;
    if let Ok(wifi) = wifi_device().await {
        disable_offload(&wifi).await;
    }

    tracing::info!(from_port, to_port, %bridge, "pfctl rules loaded via internet-sharing anchor");
    Ok(())
}

async fn disable_offload(iface: &str) {
    // Flag names vary slightly by macOS version; ignore individual failures.
    for flag in ["-tso4", "-tso6", "-lro"] {
        let _ = sh("ifconfig", &[iface, flag]).await;
    }
}

async fn enable_offload(iface: &str) {
    for flag in ["tso4", "tso6", "lro"] {
        let _ = sh("ifconfig", &[iface, flag]).await;
    }
}

pub async fn pfctl_remove() -> Result<()> {
    let _ = sh("pfctl", &["-a", "com.apple.internet-sharing", "-F", "all"]).await;
    // Also clear the global state table so stale entries don't survive into
    // the next session.
    let _ = sh("pfctl", &["-F", "states"]).await;
    tokio::fs::remove_file(PF_RULES_PATH).await.ok();

    // Restore the offloads we disabled in pfctl_add so the user's normal
    // Wi-Fi throughput isn't degraded after Unlocker is done.
    enable_offload("bridge100").await;
    if let Ok(wifi) = wifi_device().await {
        enable_offload(&wifi).await;
    }

    state::mutate(|s| s.pfctl_anchor_loaded = false).await?;
    tracing::info!("pfctl rules flushed");
    Ok(())
}

// ── DHCP leases ──────────────────────────────────────────────────────────────

pub async fn dhcpd_read() -> Result<Vec<DhcpLease>> {
    let path = "/var/db/dhcpd_leases";
    let body = match tokio::fs::read_to_string(path).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };
    Ok(parse_dhcpd_leases(&body))
}

fn parse_dhcpd_leases(s: &str) -> Vec<DhcpLease> {
    // Apple's bootpd writes "key=value" lines inside { ... } blocks.
    let mut out = Vec::new();
    let mut cur: Option<(Option<String>, Option<String>, Option<String>)> = None;
    for line in s.lines() {
        let line = line.trim();
        if line == "{" {
            cur = Some((None, None, None));
        } else if line == "}" {
            if let Some((Some(ip), Some(mac), name)) = cur.take() {
                out.push(DhcpLease { ip, mac, name });
            } else {
                cur = None;
            }
        } else if let Some((ref mut ip, ref mut mac, ref mut name)) = cur {
            if let Some(v) = line.strip_prefix("ip_address=") {
                *ip = Some(v.trim().to_string());
            } else if let Some(v) = line.strip_prefix("hw_address=") {
                // typical format: "1,aa:bb:cc:dd:ee:ff"
                let mac_only = v.trim().split(',').last().unwrap_or("").to_string();
                *mac = Some(mac_only);
            } else if let Some(v) = line.strip_prefix("name=") {
                *name = Some(v.trim().to_string());
            }
        }
    }
    out
}

// ── bridge IP discovery ──────────────────────────────────────────────────────

pub async fn bridge_ip() -> Result<String> {
    let out = sh("ifconfig", &["bridge100"]).await?;
    for line in out.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("inet ") {
            // "inet 192.168.2.1 netmask 0xffffff00 broadcast ..."
            if let Some(ip) = rest.split_whitespace().next() {
                return Ok(ip.to_string());
            }
        }
    }
    Err(anyhow!("bridge100 has no IPv4 address yet"))
}

// ── full cleanup ─────────────────────────────────────────────────────────────

pub async fn full_cleanup() -> Result<()> {
    let s = state::read().await.unwrap_or_default();

    // pfctl rules are idempotent to remove; do it unconditionally so a missing
    // state file (force-quit, fresh install over a broken prior run) still
    // tears them down.
    let _ = pfctl_remove().await;

    // Only touch NAT_PLIST when we know we wrote one (state flag) or have a
    // backup to restore from. Without either signal, the plist might be the
    // user's own Internet Sharing config — leave it alone.
    if s.internet_sharing_active || Path::new(NAT_PLIST_BACKUP).exists() {
        let _ = is_disable().await;
    }

    // Always remove our adhoc upstream service and restore lo0. This is the
    // source of the lingering loopback bug: if the service is left in
    // System Settings → Network with lo0 as upstream, macOS networkd keeps
    // tearing down 127.0.0.1 every reboot. Run regardless of state.
    remove_adhoc_upstream().await;
    restore_loopback().await;

    state::mutate(|s| {
        s.internet_sharing_active = false;
        s.pfctl_anchor_loaded = false;
    })
    .await?;
    Ok(())
}
