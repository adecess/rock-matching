# Overview

Rock matching is a simple one-asset trading engine.

At a high level, it provides two main components:

- A self-contained engine library responsible for matching incoming limit and market orders.
- An HTTP server binary that runs maker/taker bots, sends their orders to the engine and gives clients access to
  outgoing trade events through a websocket connection.

[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Apache-2.0 licensed](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE-APACHE)
[![Build Status](https://github.com/adecess/rock-matching/actions/workflows/ci.yml/badge.svg)](https://github.com/adecess/rock-matching/actions/workflows/ci.yml)

# Get started

```
git clone https://github.com/adecess/rock-matching.git
cd rock-matching
cargo run
```

# Runtime Settings

`ROCK_BIND_ADDRESS` defaults to `0.0.0.0:3000` unless specified.
`ROCK_ALLOWED_ORIGINS` contains frontend HTTPS origins, with local Vite origins as defaults.
`RUST_LOG` defaults to `info` and is optional.

The deployed frontend origin must be supplied.

⚠️ The backend's WSS URL is not an origin value ⚠️

# Websocket

`/ws` is a read-only websocket that returns the latest order book snapshot when connected and streams updates. It
requires an allowed Origin header and incoming client data messages are rejected.

# Networking

Caddy is the public facing server.