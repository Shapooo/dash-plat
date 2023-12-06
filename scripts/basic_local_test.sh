#!/bin/bash

SHELL_FOLDER=`cd $(dirname $0); pwd`
WORKSPACE_FOLDER=`cd ${SHELL_FOLDER}/..; pwd`

cd ${WORKSPACE_FOLDER}
if [ -d "experiment" ]; then
    rm -rf experiment
fi
mkdir experiment
cd experiment

cargo build
cargo run -p dash-tools --bin=config-gen

for i in {0..3}
do
    mkdir -p ${i}/config/peers
    mv $i.config.yaml ${i}/config/config.yaml
    mv $i.sec ${i}/config/sec_key
    for j in {0..3}
    do
        cp ${j}.peerconfig.yaml ${i}/config/peers/${j}.peerconfig.yaml
    done
    cp ${WORKSPACE_FOLDER}/target/debug/dash-node ${i}/
done

for i in {0..3}
do
    rm ${i}.peerconfig.yaml
done

mkdir -p client/config
mv client.config.yaml client/config/config.yaml
mv client.sec client/config/sec_key
cp ${WORKSPACE_FOLDER}/target/debug/dash-client ./client
