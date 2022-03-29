#!/bin/sh

#(cd ../import; ./dwn.sh ; cargo run)
(cd ../import; cargo run)
wrangler kv:bulk put --binding VAXKV ../import/bulk.json

