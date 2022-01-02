#!/bin/sh

(cd ../import; cargo run)
wrangler kv:bulk put --binding VAXKV ../import/bulk.json

