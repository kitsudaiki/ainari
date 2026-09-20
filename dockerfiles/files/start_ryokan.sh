#!/bin/bash

# Start wireguard, if a tunnel is configured. The local docker-compose setup has none, because
# all components share the network of the compose-project.
if [ -f /etc/wireguard/wg0.conf ]; then
    sudo wg-quick up wg0
fi

# Run ryokan
/home/ainari/ryokan
