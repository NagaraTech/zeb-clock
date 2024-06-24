# zebclock

## Overview

The zebclock is a implement of vlc(verifiable logical clock) and the zebclock is one module of zeb network.

It use the [zeb](https://github.com/hetu-project/zeb) p2p relay as network module. And as a backend project node for supporting verifiable logical clock and causality ordering. This system is currently in poc stage.

## Dependences

### PostgreDB

The zebclock depends on postgre db for data persistence, so please install postgre and setup a pg instance. 

### Zeb p2p relayer

The zebclock play a role of inner logic and state layer in vlc overview. One zebclock process matches a zeb p2p node, and them use inner net socket for communication.

For now, zebclock and zeb use the same node identity for two processes. So first generate a key pair identity, then address it to `node_id` in [config-template.yaml](./docs/config-template.yaml) of zebclock.

### Net messaging

The zebclock and zeb using protobuf proto3 as serialization compression algorithm and communication protocol. More messages body details, please see [crates/protos](../crates/protos/) for check it.

## Compile

### Build from source

```bash
git clone https://github.com/hetu-project/zeb-clock.git

cd zeb-clock

cargo build
```

## Run a node

```bash
# 1.for help info
./target/debug/zebclock -h

# 2.init db & dna business pg tables
./target/debug/zebclock --init_pg postgres://postgres:hetu@0.0.0.0:5432/vlc_inner_db

# 3.setup the node & please update the config file to your dev environment
./target/debug/zebclock --config ./docs/config-template.yaml
```

## How to test

```shell
cargo run --package zebclock --bin client_write
cargo run --package zebclock --bin client_read
```
