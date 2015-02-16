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
    Latency     0.00us    0.00us   0.00us  100.00%
    Req/Sec     0.94M    73.98k    1.03M    90.09%
  8913835 requests in 10.01s, 450.55MB read
  Socket errors: connect 0, read 0, write 0, timeout 397
Requests/sec: 890692.14
Transfer/sec:     45.02MB
```

You did read that correctly.

## License

MIT

