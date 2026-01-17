A set of `libc` hooks used to inject network latency into traffic to/from specific hosts.

The hooks themselves are defined in the `hooks` package in this repository.

```
$ cargo build -p hooks
$ LD_PRELOAD=./target/debug/libhooks.so /path/to/your/binary
```

Supposedly the hook macro ([from `redhook`](https://docs.rs/redhook/latest/redhook/)) works
with `DYLD_INSERT_LIBRARIES`/macOS:
```
$ DYLD_INSERT_LIBRARIES=target/debug/libhooks.dylib /path/to/your/binary
```

### Configuration

You can configure the hooks to an extent by setting certain environment variables:
```
$ # Only intercept sockets that connect to `github.com` or `bsky.app`.
$ #
$ # Default: Intercept sockets for all hosts.
$ export PRELOAD_LATENCY_HOSTS="github.com:bsky.app"

$ # Force hosts in `PRELOAD_LATENCY_HOSTS` to be resolved in `getaddrinfo` during
$ # program startup. Otherwise a binary that brings its own DNS resolver may not
$ # have its sockets intercepted correctly.
$ #
$ # Default: Wait for the binary to resolve hosts using `getaddrinfo` on its own.
$ export PRELOAD_LATENCY_RESOLVE=1

$ # Inject a sleep of 300 milliseconds into send/recv/related libc calls for
$ # intercepted sockets.
$ #
$ # Default: 200 milliseconds.
$ export PRELOAD_LATENCY_MILLIS=300

$ # Toggle interception from "disabled" to "enabled" every 30 seconds.
$ #
$ # Default: Unset, interception is always enabled
$ export PRELOAD_LATENCY_TOGGLE_PERIOD=30

$ # Debug the hooks themselves.
$ export RUST_LOG=hooks=trace,info

$ # Run run run
$ LD_PRELOAD=target/debug/libhooks.so /path/to/your/binary
```

### Test binary

There is a test binary in the `test-binary` package which sends a few kinds of traffic:
- HTTP traffic with `reqwest`
- HTTP traffic with `reqwest` and a `hickory-dns` resolver (bypasses `getaddrinfo`)
- gRPC traffic with `bigtable_rs`

There's a `docker-compose.yaml` which sets up a Bigtable emulator for convenience. You
can run everything like so:
```
$ docker build -f Dockerfile -t workspace .
$ docker compose up -d
$ docker exec -it preload_latency-workspace-1 /bin/bash
> cd /repo
> cargo build -p hooks
> cargo build -p test-binary
> export PRELOAD_LATENCY_HOSTS=github.com:bsky.app:bigtable
> export PRELOAD_LATENCY_RESOLVE=1
> LD_PRELOAD=target/debug/libhooks.so ./target/debug/test-binary
```
