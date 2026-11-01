# Network Overview

This page shows how the interfaces of the `network` section of the
[torii config](../config/torii_config.md) fit together.

## Interfaces

| Setting           | Gateway       | Description                                                                                                     |
| ----------------- | ------------- | --------------------------------------------------------------------------------------------------------------- |
| `overlay_iface`   | every gateway | Interface towards the VMs. Carries the plain packets of the VMs. `overlay_ingress` is attached to it.           |
| `underlay_iface`  | every gateway | Interface towards the other gateways. Carries the wrapped and encrypted traffic of the VMs. Its address is the address of the gateway, which the other gateways send the traffic to and which hanami asks for. `underlay_ingress` is attached to it. |
| `uplink_iface`    | edge only     | Interface towards the outside. The floating IPs are served there (ARP, DNAT and SNAT).                          |
| `uplink_next_hop` | edge only     | Router behind the uplink. Every packet, which leaves the virtual network, is sent to it.                        |

`uplink_iface` and `uplink_next_hop` are always set together. An internal gateway has no uplink
and uses `default_gateway_ip` instead, which is the underlay-address of the edge gateway. Both can
not be combined.

The VMs only see the overlay. The underlay is the hidden network between the hosts, which is why
the underlay-address is the one, which hanami and the other gateways have to know.

## Overview

```text
                         ┌───────────────────┐
                         │     Internet      │
                         └─────────┬─────────┘
                                   │
                         ┌─────────┴─────────┐
                         │      Router       │ ◄── uplink_next_hop = 203.0.113.1
                         │   203.0.113.1     │     (all traffic leaving the
                         └─────────┬─────────┘      virtual network goes here)
                                   │  outside network
┌──────────────────────────────────┼──────────────────────────────────┐
│ EDGE GATEWAY (torii)             │                                  │
│                                  │                                  │
│        [uplink_iface]  e.g. ens5 / uplink0                          │
│          │  • overlay_ingress attached here too                     │
│          │  • answers ARP for floating IPs with its own MAC         │
│          │  • IN : floating IP ──DNAT──► VM address                 │
│          │  • OUT: VM address  ──SNAT──► floating IP                │
│          │                                                          │
│       ┌──┴──────────────────────┐                                   │
│       │      eBPF datapath      │  default route 0.0.0.0            │
│       │                         │  ──► uplink_next_hop              │
│       └──┬──────────────────────┘                                   │
│          │                                                          │
│        [underlay_iface]  10.1.0.1                                   │
└──────────┼──────────────────────────────────────────────────────────┘
           │
═══════════╪═════════════════════════════════════════════════════════════
  UNDERLAY NETWORK (real host network) – only wrapped/encrypted VM traffic
═══════╤═══════════════════════════════════════════════╤═════════════════
       │                                               │
┌──────┴──────────────────────────┐   ┌────────────────┴────────────────┐
│ HOST A – internal gateway       │   │ HOST B – internal gateway       │
│ (no uplink_iface)               │   │ (no uplink_iface)               │
│                                 │   │                                 │
│ default_gateway_ip = 10.1.0.1   │   │ default_gateway_ip = 10.1.0.1   │
│ ("unknown dest? send to edge")  │   │                                 │
│                                 │   │                                 │
│ [underlay_iface] 10.1.0.2       │   │ [underlay_iface] 10.1.0.3       │
│   │ underlay_ingress: unwrap    │   │   │ underlay_ingress: unwrap    │
│ ┌─┴────────────────┐            │   │ ┌─┴────────────────┐            │
│ │  eBPF datapath   │            │   │ │  eBPF datapath   │            │
│ └─┬────────────────┘            │   │ └─┬────────────────┘            │
│   │ overlay_ingress: ARP,       │   │   │                             │
│   │ filter, tenant (VNI)        │   │   │                             │
│ [overlay_iface] / TAPs          │   │ [overlay_iface] / TAPs          │
│   │           │                 │   │   │                             │
│ ┌─┴──┐      ┌─┴──┐              │   │ ┌─┴──┐                          │
│ │VM 1│      │VM 2│              │   │ │VM 3│                          │
│ └────┘      └────┘              │   │ └────┘                          │
└─────────────────────────────────┘   └─────────────────────────────────┘
```

## Packet paths

### VM to VM on another host

```text
VM 1 ──plain──► overlay_iface (A) ──► eBPF: "VM 3 is behind 10.1.0.3"
     ──► wrap/encrypt, source = underlay-address of A (10.1.0.2)
     ──► underlay_iface (A) ═══ underlay ═══► underlay_iface (B)
     ──► unwrap ──► TAP of VM 3 ──plain──► VM 3
```

### VM to the internet

```text
VM 1 ──► overlay_iface (A) ──► no route ──► default_gateway_ip 10.1.0.1
     ══ underlay ══► underlay_iface (edge) ──► unwrap
     ──► SNAT: source = floating IP ──► uplink_iface ──► uplink_next_hop ──► Internet
```

### Internet to the floating IP of a VM

```text
Internet ──► router ──► ARP "who has 198.51.100.7?" ◄── edge answers with the uplink MAC
     ──► uplink_iface ──► DNAT: floating IP → address of VM 3
     ──► wrap ──► underlay_iface (edge) ══ underlay ══► underlay_iface (B)
     ──► unwrap ──► TAP ──► VM 3
```

## Single node setup

With `development.single_node = true` one torii does everything. The floating IPs are translated
directly at the uplink, so only the uplink settings are required.

```text
 Internet ── Router (uplink_next_hop) ── [uplink_iface] ── torii ── TAPs ── VMs
                                                            │
                                 overlay_iface  = "none"    │  no underlay,
                                 underlay_iface = "none"    │  no tunnel, no IPsec
```
