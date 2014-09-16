# Event [![Build Status](https://travis-ci.org/reem/rust-event.svg?branch=master)](https://travis-ci.org/reem/rust-event)

> A focused API providing a multi-threaded, asynchronous event queue.

## Overview

A multi-threaded Event-Loop for Rust.

This event loop supports only events which originate from Rust, but can be used
as a global event queue for inter-task and asynchronous communication.

Rust-Event exports a small and focused API:

```rust
fn queue() -> Ref<Arc<EventQueue>>;

struct Event<T>;

impl<T: Send> Event<T> {
  fn new(val: T) -> Event<T>;
  fn trigger<K: Assoc<T>>(self);
}

type Handler<T> = Box<Fn<(Box<Event<T>>,), ()> + Send;

fn on<K: Assoc<T>, T: Send>(handler: Handler<T>);

fn dedicate();

impl EventQueue {
  fn trigger(&self);
}

fn spawn(proc(): Send);
```

It has one method for triggering events - `Event::trigger`, and one
method for listening for events `on`. Threads can offer themselves
to be used to continuously handle events using `dedicate`, which will
block the thread, causing it to spend all of its time consuming events
from the global EventQueue.

Threads can try to handle a single event through `EventQueue::trigger`.

`spawn` is used to spawn tasks which have access to the `EventQueue`.

That's it! Further abstractions should be built above this basic API.

## Example:

```rust
use event::{mod, EventQueue, Event};
use typemap::Assoc;

struct NewConnection;
struct Connection;

impl Assoc<Connection> for NewConnection {}

// Create the EventQueue
EventQueue::new();

// Listen for NewConnection events.
event::on::<NewConnection>(|&: conn| /* something with conn */);

event::spawn(proc() {
    // Queues this event to be fired.
    Event::new(Connection).trigger::<NewConnection>();
});

// Get an event off the queue and handle it in this thread, if there are any.
event::queue().trigger();

// Dedicate this thread to handling events, blocking it.
event::dedicate()
```

## License

MIT

