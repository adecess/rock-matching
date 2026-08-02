# Overview

---
Rock matching is a simple one-asset trading engine.

At a high level, it provides two main components:

- A self-contained engine library responsible for matching incoming limit and market orders.
- An HTTP server that runs maker/taker bots and gives clients access to outgoing trade events through a websocket
  connection.

[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Apache-2.0 licensed](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE-APACHE)
[![Build Status](https://github.com/adecess/rock-matching/actions/workflows/ci.yml/badge.svg)](https://github.com/adecess/rock-matching/actions/workflows/ci.yml)

# Get started

```
git clone https://github.com/adecess/rock-matching.git
cd rock-matching
cargo run
```
