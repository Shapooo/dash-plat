# Config File Documentation

Running dash-node requires pre-configuring the config file folder. If not specified, it defaults to the config folder in the same directory as the dash-node executable. Sample directory structure and description are as follows:

```
config
├── config.yaml // Main config file
├── peers // Contains config files for all peers (including self), file names can be arbitrary
│   ├── 0.yaml
│   ├── 1.yaml
│   ├── 2.yaml
│   ├── 3.yaml
│   └── ...
└── sec_key // Node's ed25519 private key in PEM format, can be generated by tools
```

## Main Config File Description

The main config file needs to contain the following:

```
# dash-node listening address and TCP port for peers
host_address: 127.0.0.1:8080

# dash-node listening address and TCP port for client
client_listen_address: 127.0.0.1:8081

# View timeout, unit milliseconds: waiting time before current view timeout
minimum_view_timeout_ms: 500

# Limit on number of blocks requested from sync peer per response when syncing
sync_request_limit: 10

# Sync response timeout, unit milliseconds
sync_response_timeout_ms: 5000
```

## Peer Config File Description

Description is as follows

```
# Peer node address and port
host_addr: 127.0.0.1:8080

# Public key of peer node
public_key: db3MWGjrGbXuxXyLCU02rh/MyowpwfHIh8etJF5wVmI=
```
