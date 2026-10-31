#!/bin/sh

set -x

systemctl list-units --type=service | grep moblin2jpnkn
systemctl list-unit-files | grep moblin2jpnkn

# Check if moblin2jpnkn is installed and running
systemctl is-active --quiet moblin2jpnkn && echo "moblin2jpnkn is running" || echo "moblin2jpnkn is not running"