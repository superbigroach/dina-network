#!/bin/bash
TOKEN=$(curl -s -H "Metadata-Flavor: Google" "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token" | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])" 2>/dev/null)
[ -n "$TOKEN" ] && echo "$TOKEN" | docker login -u oauth2accesstoken --password-stdin us-central1-docker.pkg.dev 2>/dev/null
docker pull us-central1-docker.pkg.dev/lucilla-b0493/dina-network/dina-node:latest
docker stop dina-node 2>/dev/null; docker rm dina-node 2>/dev/null
rm -f /home/chronos/dina-data/chain.redb 2>/dev/null
chmod 666 /home/chronos/dina-data/node_key 2>/dev/null
chmod 777 /home/chronos/dina-data 2>/dev/null
docker run -d --name dina-node --restart=unless-stopped \
  -p 8080:8080 -p 8545:8545 -p 9000:9000 \
  -v /home/chronos/dina-data:/data \
  us-central1-docker.pkg.dev/lucilla-b0493/dina-network/dina-node:latest \
  --data-dir /data --rpc-bind 0.0.0.0 --listen /ip4/0.0.0.0/tcp/9000 \
  --validator \
  --bootstrap /ip4/35.184.213.248/tcp/9000 \
  --bootstrap /ip4/34.118.177.132/tcp/9000 \
  --bootstrap /ip4/35.246.48.82/tcp/9000 \
  --bootstrap /ip4/136.109.115.69/tcp/9000
echo "DEPLOYED $(date)" > /tmp/deploy-status
