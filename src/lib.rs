#![feature(macro_rules)]
#![license = "MIT"]
#![deny(missing_docs)]
#![deny(warnings)]

//! A multi-threaded Event-Loop for Rust.
//!
//! This event loop supports only events which originate from Rust, but can be used
//! as a global event queue for inter-task and asynchronous communication.
//!
//! Rust-Event exports a small and focused API:
//!
//! ```{notrust, ignore}
//! fn queue() -> Ref<Arc<EventQueue>>;
//!
//! struct Event<T>;
//!
//! impl<T: Send> Event<T> {
//!   fn new(val: T) -> Event<T>;
//!   fn trigger<K: Assoc<T>>(self);
//! }
//!
//! type Handler<T> = Box<Fn<(Box<Event<T>>,), ()> + Send;
//!
//! fn on<K: Assoc<T>, T: Send, F: /* unboxed Handler<T> */>(handler: F);
//!
//! fn dedicate();
//!
//! impl EventQueue {
//!   fn trigger(&self);
//! }
//!
//! fn spawn(proc(): Send);
//! ```
//!
//! It has one method for triggering events - `Event::trigger`, and one
//! method for listening for events `on`. Threads can offer themselves
//! to be used to continuously handle events using `dedicate`, which will
//! block the thread, causing it to spend all of its time consuming events
//! from the global EventQueue.
//!
//! Threads can try to handle a single event through `EventQueue::trigger`.
//!
//! `spawn` is used to spawn tasks which have access to the `EventQueue`.
//!
//! That's it! Further abstractions should be built above this basic API.
//!

extern crate typemap;
extern crate "unsafe-any" as uany;
extern crate rustrt;

use std::sync::{Arc, Mutex, RWLock};
use std::any::Any;
use std::intrinsics::TypeId;
use std::mem;

use rustrt::local_data::Ref;

use typemap::{TypeMap, Assoc};
use uany::{UncheckedAnyDowncast, UncheckedBoxAnyDowncast};

local_data_key!(LocalEventQueue: Arc<EventQueue>)

/// Get an immutable reference to the EventQueue stored in TLS.
///
/// ## Failure
///
/// Fails if called from a task which does not have an EventQueue in
/// TLS.
///
/// Tasks with an EventQueue can be created using event's custom `spawn`.
pub fn queue() -> Ref<Arc<EventQueue>> {
    LocalEventQueue.get().expect("an Event Queue to be in TLS")
}

/// An Event containing arbitrary data.
pub struct Event<T: Send>(T);

impl<T: Send> Event<T> {
    /// Construct a new Event.
    pub fn new(val: T) -> Event<T> {
        Event(val)
    }

    /// Trigger this a named event with this Event as the associated data.
    pub fn trigger<K: Assoc<T>>(self) {
        queue().queue::<K, T>(self)
    }
}

/// Register a Handler for Events of type K.
pub fn on<K: Assoc<X>, X: Send>(handler: Handler<X>) {
    queue().on::<K, X>(handler)
}

/// Handlers are Fns of a specific type.
pub type Handler<X> = Box<Fn<(Box<X>,), ()> + Send>;

enum EventKey<K: Assoc<X>, X: Send> {}

impl<K: Assoc<X>, X: Send> Assoc<Handler<X>> for EventKey<K, X> {}

/// The global event-queue, which collects and dispatches events.
pub struct EventQueue {
    // The TypeId here is the key of the Handler in TypeMap,
    // in this case it is TypeId::of::<EventKey<K, X>> and the
    // Box<Any> is Event<X>.
    //
    // That way - even though we do not know X, we can do an
    // unchecked downcast of the Handler to a &Fn<(&(),), ()>
    // and downcast the Event to Event<()>, access the data, box it,
    // and call the Handler with the data.
    //
    // Receiver is NoSync, so we must use a Mutex.
    queue: Mutex<Receiver<(TypeId, Box<Any + Send>)>>,

    // Since we usually need only immutable access to call Handlers, we can use an RWLock
    // to reduce contention.
    handlers: RWLock<TypeMap>,

    // The sender used to push onto the queue.
    //
    // Sender is NoSync, so we must use a Mutex.
    enqueuer: Mutex<Sender<(TypeId, Box<Any + Send>)>>
}

impl EventQueue {
    /// Create a new EventQueue, inserting it into TLS.
    pub fn new() -> Arc<EventQueue> {
        let (sender, receiver) = channel();
        let this = Arc::new(EventQueue {
            queue: Mutex::new(receiver),
            handlers: RWLock::new(TypeMap::new()),
            enqueuer: Mutex::new(sender)
        });

        // Put ourselves in TLS.
        LocalEventQueue.replace(Some(this.clone()));
        this
    }

    fn queue<K: Assoc<X>, X: Send>(&self, event: Event<X>) {
        self.enqueuer.lock().send((TypeId::of::<EventKey<K, X>>(), box event as Box<Any + Send>));
    }

    /// Triggers the first event in the queue, running the handler in the calling thread.
    ///
    /// If there is no event, this blocks the current task until there is one.
    pub fn trigger(&self) {
        // Lock the queue for as short a time as possible, just grabbing an event if there is one.
        let (id, event) = self.queue.lock().recv();

        // We need the read lock as long as we have handler.
        let read = self.handlers.read();

        let handler = unsafe {
            // Grab the internal HashMap from TypeMap and query for the data
            // with the TypeId we got from the queue.
            //
            // This should always contain a Handler of the appropriate type as we are
            // careful to only allow insertions of (EventKey<K, X>, Handler<X>) pairs.
            match read.data().get(&id) {
                Some(x) => x,
                // No handler for this event, move along.
                None => return
            // Downcast this handler to the fake type, which takes an opaque pointer
            // instead of the correct Box<Event<X>> because we cannot name X.
            }.downcast_ref_unchecked::<&Fn<(*mut (),), ()>>()
        };

        // Get the data as an opaque pointer. This is highly, highly unsafe but is fine
        // here because we carefully only inserted the correct event type for the correct
        // handler.
        //
        // This is necessary because we cannot name the real type behind this pointer here,
        // as that information was erased when we turned the Event into a Box<Any>.
        let data = unsafe { mem::transmute::<Box<()>, *mut ()>(event.downcast_unchecked()) };

        // Call the handler with an opaque pointer to the data. Since Box<T> and *mut ()
        // have the same representation the Handler's code can actually treat the data as
        // the type it expects, and all is good.
        //
        // Additionally, since on our end the data is *mut (), we do not run a destructor
        // and the Handler is free to (and will) drop the data.
        handler.call((data,));
    }

    fn on<K: Assoc<X>, X: Send>(&self, handler: Handler<X>) {
        // This is the only time we take a write lock, because we have to insert.
        self.handlers.write().insert::<EventKey<K, X>, Handler<X>>(handler);
    }
}

/// Spawn a task which can access the EventQueue.
pub fn spawn(func: proc(): Send) {
    use std::task;

    let queue = queue().clone();
    task::spawn(proc() {
        LocalEventQueue.replace(Some(queue));
        func()
    });
}

/// Block this task and dedicate it to handling events.
pub fn dedicate() {
    let queue = queue();
    loop { queue.trigger() }
}

