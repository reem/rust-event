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

## License

MIT

