//! Small helpers around the network configuration of the host.
//!
//! These wrap the pieces of system state the gateway has to read or change -
//! interface indices, MAC addresses, neighbour resolution and the `ip` command
//! itself - so the endpoints and the routing logic stay readable.

use std::net::Ipv4Addr;

use ainari_common::secret::Secret;

/// Retrieves the system index of a network interface.
///
/// This function reads the `/sys/class/net/{name}/ifindex` file to resolve the
/// numeric interface index used by the kernel and eBPF.
///
/// # Arguments
/// * `name` - The name of the network interface (e.g., "eth0")
///
/// # Returns
/// A `u32` representing the interface index, or 0 if not found
pub fn get_ifindex(name: &str) -> u32 {
    let path = format!("/sys/class/net/{}/ifindex", name);
    std::fs::read_to_string(path)
        .unwrap_or_else(|_| "0\n".to_string())
        .trim()
        .parse()
        .unwrap_or(0)
}

/// Retrieves the MAC address of a network interface.
///
/// This function reads the `/sys/class/net/{iface}/address` file and parses
/// the hex string into a standard 6-byte array.
///
/// # Arguments
/// * `iface` - The name of the network interface
///
/// # Returns
/// A 6-byte array `[u8; 6]` containing the MAC address. Defaults to zeros on failure.
pub fn get_mac_address(iface: &str) -> [u8; 6] {
    let path = format!("/sys/class/net/{}/address", iface);
    let mac_str = std::fs::read_to_string(path).unwrap_or_else(|_| "00:00:00:00:00:00".to_string());
    let mut mac = [0u8; 6];
    for (i, byte) in mac_str.trim().split(':').enumerate() {
        if i < 6 {
            mac[i] = u8::from_str_radix(byte, 16).unwrap_or(0);
        }
    }
    mac
}

/// Retrieves the primary IPv4 address of a local network interface.
///
/// This function executes the `ip -4 addr show` command and parses the output
/// to locate the first available inet address assigned to the interface.
///
/// # Arguments
/// * `iface` - The name of the network interface
///
/// # Returns
/// An `Option<Ipv4Addr>` containing the IP address, or None if unavailable.
pub fn get_local_ip(iface: &str) -> Option<Ipv4Addr> {
    let output = std::process::Command::new("ip")
        .arg("-4")
        .arg("addr")
        .arg("show")
        .arg(iface)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("inet ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].split('/').next()?.parse().ok();
            }
        }
    }
    None
}

/// Determines the link layer next hop towards an address.
///
/// The underlay does not have to be a single link: on a routed network, like
/// the pod-network of most kubernetes-clusters, the other gateways are only
/// reachable over a router. The kernel knows which one, so its route towards
/// the address is asked, restricted to the given interface.
///
/// # Arguments
/// * `ip` - The address, which has to be reached
/// * `iface` - The interface, which the traffic leaves on
///
/// # Returns
/// The gateway of the route, or `ip` itself, if it is directly on the link or
/// the kernel has no route
pub fn get_next_hop(ip: Ipv4Addr, iface: &str) -> Ipv4Addr {
    let output = std::process::Command::new("ip")
        .args(["-4", "route", "get", &ip.to_string(), "oif", iface])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            parse_route_via(&String::from_utf8_lossy(&output.stdout)).unwrap_or(ip)
        }
        _ => ip,
    }
}

/// Extracts the gateway from the output of `ip route get`.
///
/// # Arguments
/// * `route` - Output of `ip route get`, like `10.244.1.5 via 10.244.0.1 dev eth0 src ...`
///
/// # Returns
/// The address behind `via`, or `None` for a destination on the link itself
fn parse_route_via(route: &str) -> Option<Ipv4Addr> {
    let mut words = route.split_whitespace();
    while let Some(word) = words.next() {
        if word == "via" {
            return words.next()?.parse().ok();
        }
    }
    None
}

/// Resolves the MAC address of a target IP using ARP.
///
/// This function pings the target IP to force an ARP resolution, then parses
/// the `/proc/net/arp` table to find the corresponding MAC address. It retries
/// up to 10 times to allow network convergence.
///
/// # Arguments
/// * `ip` - The target IPv4 address
///
/// # Returns
/// A 6-byte array `[u8; 6]` containing the resolved MAC address, or a broadcast address (0xff) on failure.
pub fn get_arp_mac(ip: Ipv4Addr) -> [u8; 6] {
    let ip = ip.to_string();
    for _ in 0..10 {
        std::process::Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg("-W")
            .arg("1")
            .arg(&ip)
            .output()
            .ok();
        if let Ok(arp_table) = std::fs::read_to_string("/proc/net/arp") {
            for line in arp_table.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 && parts[0] == ip {
                    let mac_str = parts[3];
                    if mac_str != "00:00:00:00:00:00" {
                        let mut mac = [0u8; 6];
                        for (i, byte) in mac_str.split(':').enumerate() {
                            if i < 6 {
                                mac[i] = u8::from_str_radix(byte, 16).unwrap_or(0);
                            }
                        }
                        return mac;
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    [0xff; 6]
}

/// Runs an `ip` command and turns a non-zero exit status into an error.
///
/// The gateway drives the kernel's routing, neighbour and xfrm tables through
/// iproute2 instead of talking netlink directly, which keeps the individual
/// operations readable and easy to reproduce by hand when debugging.
///
/// # Arguments
/// * `args` - The argument vector handed to the `ip` binary
///
/// # Returns
/// `Ok(())` when the command succeeded, otherwise the captured stderr
pub fn run_ip(args: &[&str]) -> Result<(), String> {
    run_ip_with_secrets(args, &[])
}

/// Runs an `ip` command whose arguments contain secret values.
///
/// The error message of a failed command repeats the full command line, and
/// iproute2 may echo arguments on stderr as well. Every secret is therefore
/// masked in the message before it is handed back to the caller, which usually
/// forwards it into logs or an HTTP response.
///
/// # Arguments
/// * `args` - The argument vector handed to the `ip` binary
/// * `secrets` - The secrets revealed somewhere inside `args`
///
/// # Returns
/// `Ok(())` when the command succeeded, otherwise the masked error message
pub fn run_ip_with_secrets(args: &[&str], secrets: &[&Secret]) -> Result<(), String> {
    let output = std::process::Command::new("ip")
        .args(args)
        .output()
        .map_err(|e| mask_secrets(format!("failed to run ip {:?}: {}", args, e), secrets))?;

    if output.status.success() {
        return Ok(());
    }
    Err(mask_secrets(
        format!(
            "ip {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        secrets,
    ))
}

/// Replaces every occurrence of the given secrets in a message by their masked form.
///
/// # Arguments
/// * `message` - The message that may contain secret values
/// * `secrets` - The secrets to mask
///
/// # Returns
/// The message without any of the secret values
fn mask_secrets(mut message: String, secrets: &[&Secret]) -> String {
    for secret in secrets {
        if !secret.reveal().is_empty() {
            message = message.replace(secret.reveal(), &secret.to_string());
        }
    }
    message
}

/// Enables IPv4 forwarding for the given sysctl path, ignoring missing knobs.
///
/// Traffic that is protected by IPsec leaves the eBPF datapath and is routed by
/// the kernel, which only happens when forwarding is switched on.
///
/// # Arguments
/// * `path` - The sysctl file below `/proc/sys` to write a `1` into
///
/// # Returns
/// None. Failures are ignored on purpose - the knob simply may not exist.
pub fn enable_forwarding(path: &str) {
    let _ = std::fs::write(path, b"1");
}

/// Parses a textual MAC address into its raw 6-byte representation.
///
/// # Arguments
/// * `mac` - A MAC address in the usual colon separated hex notation
///
/// # Returns
/// An `Option<[u8; 6]>` holding the parsed address, or None if malformed
pub fn parse_mac(mac: &str) -> Option<[u8; 6]> {
    let mut out = [0u8; 6];
    let mut seen = 0;
    for (i, part) in mac.trim().split(':').enumerate() {
        if i >= 6 {
            return None;
        }
        out[i] = u8::from_str_radix(part, 16).ok()?;
        seen += 1;
    }
    if seen != 6 {
        return None;
    }
    Some(out)
}

/// Rejects a VNI, which would not survive the 24 bit field of a VXLAN header.
///
/// Everything else about a tenant is implicit - a tenant exists as soon as something is
/// registered in it - so this is the only validation a VNI ever needs.
///
/// # Arguments
/// * `vni` - The tenant taken from a request-payload
///
/// # Returns
/// `Ok(())` for a usable tenant, otherwise a message naming the limit
pub fn validate_vni(vni: u32) -> Result<(), String> {
    if vni > torii_common::VNI_MAX {
        return Err(format!(
            "vni {} is larger than the 24 bit VXLAN limit {}",
            vni,
            torii_common::VNI_MAX
        ));
    }
    Ok(())
}

/// Priority of the rule, which sends the traffic of an interface into the table of its tenant.
const RULE_PRIO_TABLE: &str = "1000";

/// Priority of the rule, which stops the lookup when the table of the tenant has no answer.
const RULE_PRIO_GUARD: &str = "1001";

/// Appends `table <id>` to an `ip route` argument-vector, when the tenant has one.
///
/// # Arguments
/// * `args` - The argument-vector, which is being built
/// * `table` - The table-name returned by `Network::tenant_table`
///
/// # Returns
/// None. The vector is extended in place.
pub fn with_table<'a>(args: &mut Vec<&'a str>, table: &'a Option<String>) {
    if let Some(table) = table.as_deref() {
        args.extend_from_slice(&["table", table]);
    }
}

/// Points everything arriving on one interface at the routing-table of its tenant.
///
/// This is the kernel side counterpart of the VNI: a packet, which XDP handed up for IPsec
/// processing, has lost every trace of its tenant by then, and the only thing left to recognise
/// it by is the interface it came in on.
///
/// Two rules are written, not one. A plain table-rule falls through to the next rule, when its
/// table has no matching route, and the next rule is eventually `main` - which is the table of
/// the shared tenant. A tenant, which does not know a destination, would therefore quietly borrow
/// the route of tenant 0, so a second rule right behind the first one ends the lookup instead.
///
/// The rules are removed first, because `ip rule` has no replace-operation and would happily
/// install the same rule twice.
///
/// # Arguments
/// * `iface` - The interface the rules select on
/// * `table` - The table-name returned by `Network::tenant_table`
///
/// # Returns
/// `Ok(())` when both rules are in place, or the error reported by `ip`
pub fn bind_iface_to_table(iface: &str, table: &Option<String>) -> Result<(), String> {
    let Some(table) = table.as_deref() else {
        return Ok(());
    };
    unbind_iface_from_table(iface, &Some(table.to_owned()));
    run_ip(&[
        "rule",
        "add",
        "iif",
        iface,
        "table",
        table,
        "priority",
        RULE_PRIO_TABLE,
    ])?;
    run_ip(&[
        "rule",
        "add",
        "iif",
        iface,
        "unreachable",
        "priority",
        RULE_PRIO_GUARD,
    ])
}

/// Drops the policy-routing rules of an interface again.
///
/// # Arguments
/// * `iface` - The interface the rules select on
/// * `table` - The table-name returned by `Network::tenant_table`
///
/// # Returns
/// None. Errors are ignored: the rules may already be gone.
pub fn unbind_iface_from_table(iface: &str, table: &Option<String>) {
    if let Some(table) = table.as_deref() {
        let _ = run_ip(&[
            "rule",
            "del",
            "iif",
            iface,
            "table",
            table,
            "priority",
            RULE_PRIO_TABLE,
        ]);
        let _ = run_ip(&[
            "rule",
            "del",
            "iif",
            iface,
            "unreachable",
            "priority",
            RULE_PRIO_GUARD,
        ]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_is_only_appended_for_a_real_tenant() {
        let mut args = vec!["route", "replace", "10.0.0.1/32"];
        with_table(&mut args, &None);
        assert_eq!(args, vec!["route", "replace", "10.0.0.1/32"]);

        let table = Some("101".to_owned());
        with_table(&mut args, &table);
        assert_eq!(
            args,
            vec!["route", "replace", "10.0.0.1/32", "table", "101"]
        );
    }

    #[test]
    fn the_gateway_of_a_routed_destination_is_the_next_hop() {
        let route = "10.244.1.5 via 10.244.0.1 dev eth0 src 10.244.0.23 uid 0 \n    cache \n";
        assert_eq!(parse_route_via(route), Some(Ipv4Addr::new(10, 244, 0, 1)));
    }

    #[test]
    fn a_destination_on_the_link_has_no_gateway() {
        let route = "172.30.0.10 dev eth0 src 172.30.0.20 uid 0 \n    cache \n";
        assert_eq!(parse_route_via(route), None);
    }

    #[test]
    fn secrets_are_masked_in_error_messages() {
        let key = Secret::from("0xdeadbeef");
        let message = mask_secrets(
            "ip xfrm state add 0xdeadbeef 128 failed: 0xdeadbeef".to_string(),
            &[&key],
        );
        assert_eq!(message, "ip xfrm state add *** 128 failed: ***");
    }

    #[test]
    fn a_failing_command_does_not_leak_its_secrets() {
        let key = Secret::from("0xdeadbeef");
        let err =
            run_ip_with_secrets(&["this-is-no-ip-object", key.reveal()], &[&key]).unwrap_err();
        assert!(!err.contains(key.reveal()), "{}", err);
    }
}
