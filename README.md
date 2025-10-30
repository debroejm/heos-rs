# HEOS-rs

![License](https://img.shields.io/badge/license-BSD--2--clause%20Plus%20Patent-blue.svg)
[![Crates.io](https://img.shields.io/crates/v/heos.svg)](https://crates.io/crates/heos)
[![Docs](https://docs.rs/heos/badge.svg)](https://docs.rs/heos/latest/heos/)

These bindings are based on the published HEOS control protocol specifications found on Denon's 
website.

At the time of writing, the latest version is 1.17, found here:
https://rn.dmglobal.com/usmodel/HEOS_CLI_ProtocolSpecification-Version-1.17.pdf

If that links gets stale and no longer works, a newer version may be able to be found here:
https://support.denon.com/app/answers/detail/a_id/6953/~/heos-control-protocol-%28cli%29

## Getting Started

```rust
use heos::{ConnectError, HeosConnection};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ConnectError> {
    let connection = HeosConnection::connect_any(Duration::from_secs(10)).await?
        .init_stateful().await?;
    
    for playable in connection.playables().await {
        // Do something
    }
    
    Ok(())
}
```

## Async Compatibility
This library uses [Tokio](https://tokio.rs/) under the hood in order to spawn asynchronous tasks, as
well as manage IO via e.g. TCP sockets. All async methods and functions need to be called from 
within a Tokio runtime. If you are using smol/async-std, you can use
[async-compat](https://crates.io/crates/async-compat) in order to wrap your futures with a layer
that provides a Tokio runtime.

## Roadmap

* Attempt to add support for WASM
* Improve the HEOS Control app