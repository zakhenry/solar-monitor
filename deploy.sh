#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

readonly TARGET_HOST=zak@solarmonitor.local
readonly TARGET_PATH=/home/zak/solar-monitor
# requires `brew install arm-unknown-linux-gnueabihf`
readonly TARGET_ARCH=arm-unknown-linux-gnueabihf
readonly SOURCE_PATH=./target/${TARGET_ARCH}/release/solar-monitor
readonly SYSTEMD_SERVICE=solar-monitor.service

cargo build --release --target=${TARGET_ARCH} --features i2c_display
rsync ${SOURCE_PATH} ${TARGET_HOST}:${TARGET_PATH}
rsync ./${SYSTEMD_SERVICE} ${TARGET_HOST}:/home/zak/${SYSTEMD_SERVICE}
ssh -t ${TARGET_HOST} "(pkill --signal SIGINT solar-monitor || true) && sudo cp solar-monitor.service /lib/systemd/system/solar-monitor.service && sudo systemctl daemon-reload && sudo systemctl enable solar-monitor.service && ${TARGET_PATH}"
