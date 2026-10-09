#!/bin/bash
#
# Keeps the peers of the wireguard-interface of the pod in sync with the peers in
# /etc/ainari/wireguard/$POD_NAME.peers, which has one line '<public key> <allowed ips>
# <endpoint>' per peer. The interface itself is created by the component at its start without
# any peer, so it never fails because of a peer, which doesn't exist yet. The secret with the
# peers is updated by the operator, when the number of replicas changes, so new pods are added
# and removed ones are dropped without a restart of this pod. The endpoints are names of pods,
# which are resolved again in every round, so a recreated pod with a new address is found.
PEERS="/etc/ainari/wireguard/$POD_NAME.peers"

while true; do
    if wg show wg0 > /dev/null 2>&1 && [ -f "$PEERS" ]; then
        wanted=" "
        while read -r key allowed_ips endpoint; do
            [ -n "$key" ] || continue
            wanted="$wanted$key "
            # a peer, whose name can't be resolved yet, is added without endpoint, so it can
            # still connect itself
            wg set wg0 peer "$key" allowed-ips "$allowed_ips" endpoint "$endpoint" persistent-keepalive 25 2> /dev/null \
                || wg set wg0 peer "$key" allowed-ips "$allowed_ips" persistent-keepalive 25
        done < "$PEERS"

        for key in $(wg show wg0 peers); do
            case "$wanted" in
                *" $key "*) ;;
                *) wg set wg0 peer "$key" remove ;;
            esac
        done
    fi
    sleep 10
done
