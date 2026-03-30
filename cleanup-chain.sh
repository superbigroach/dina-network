#!/bin/bash
# One-time cleanup: remove old chain data so the node starts fresh
rm -f /home/chronos/dina-data/chain.redb
echo "Chain data cleared at $(date)" > /tmp/cleanup-done
