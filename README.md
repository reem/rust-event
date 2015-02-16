# Event [![Build Status](https://travis-ci.org/reem/rust-event.svg?branch=master)](https://travis-ci.org/reem/rust-event)

> A fast, thread-local event loop.

## Overview

This is a fully-featured event loop built on top of `mio` that provides
IO, timeout, and "next tick" listeners. It is meant to expose nearly all
of the flexibility and speed of mio while making huge ergonomic gains by
internally managing handler registration.

## Examples

There are examples in the `examples` folder that show the functionality of
`event`.

## Benchmarks

A simple wrk benchmark of the tcp example\*:

```sh
$ ./target/release/examples/tcp&
$ wrk -t12 -c100 -d10s http://localhost:3000/
Running 10s test @ http://localhost:3000/
  12 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     0.90ms  233.11us   2.33ms   88.39%
    Req/Sec    42.29k     4.51k   52.89k    73.75%
  4825633 requests in 10.00s, 243.91MB read
Requests/sec: 482629.32
Transfer/sec:     24.39MB
```

Ya, it's fast.

\* My machine only, YMMV

## License

MIT

