#!/bin/bash

DEST=$1
FIRMWARE=$2
SRST=23
FWSPICK=18

echo "PULL LOW ON SRST"
pinctrl set ${SRST} op dl
echo "PULL HIGH ON FWSPICK"
pinctrl set ${FWSPICK} op pn dh
sleep 1
echo "PULL HIGH ON SRST"
pinctrl set ${SRST} op dh
sleep 1

echo "Sending firmware ${FIRMWARE} to ${DEST}"
pv ${FIRMWARE} | socat -u STDIN ${DEST}
