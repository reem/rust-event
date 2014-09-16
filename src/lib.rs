#![feature(macro_rules)]
#![license = "MIT"]
//#![deny(missing_doc)]
#![deny(warnings)]

//! Crate comment goes here

extern crate typemap;
extern crate "unsafe-any" as uany;
extern crate rustrt;

use std::collections::{RingBuf, Deque};
use std::sync::{Arc, Mutex, RWLock};
use std::any::Any;
use std::intrinsics::TypeId;

use rustrt::local_data::Ref;

use typemap::{TypeMap, Assoc};
use uany::UncheckedAnyDowncast;

local_data_key!(LocalEventQueue: Arc<EventQueue>)

pub fn queue() -> Ref<Arc<EventQueue>> {
    LocalEventQueue.get().expect("an Event Queue to be in TLS")
}

pub struct Event<T: Send>(T);

impl<T: Send> Event<T> {
    pub fn new(val: T) -> Event<T> {
        Event(val)
    }

    pub fn trigger<K: Assoc<T>>(self) {
        queue().queue::<K, T>(self)
    }
}

pub fn on<K: Assoc<X>, X: Send>(handler: Handler<X>) {
    queue().on::<K, X>(handler)
}

pub type Handler<X> = Box<Fn<(Box<Event<X>>,), ()> + Send>;

enum EventKey<K: Assoc<X>, X: Send> {}

impl<K: Assoc<X>, X: Send> Assoc<Handler<X>> for EventKey<K, X> {}

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
    // Since we always need a mutable lock, we use Mutex for faster locking and unlocking.
    queue: Mutex<RingBuf<(TypeId, Box<Any + Send>)>>,

    // Since we usually need only immutable access to call Handlers, we can use an RWLock
    // for more concurrent access.
    handlers: RWLock<TypeMap>
}

impl EventQueue {
    pub fn new() -> Arc<EventQueue> {
        let this = Arc::new(EventQueue {
            queue: Mutex::new(RingBuf::new()),
            handlers: RWLock::new(TypeMap::new())
        });

        // Put ourselves in TLS.
        LocalEventQueue.replace(Some(this.clone()));
        this
    }

    fn queue<K: Assoc<X>, X: Send>(&self, event: Event<X>) {
        // Lock the queue for the shortest time possible, just pushing the data.
        self.queue.lock().push((TypeId::of::<EventKey<K, X>>(), box event as Box<Any + Send>));
    }

    /// Triggers the first event in the queue, running the handler in the calling thread.
    ///
    /// If there is no event, this returns immediately.
    pub fn trigger(&self) {
        // Lock the queue for as short a time as possible, just grabbing an event if there is one.
        let (id, event) = match self.queue.lock().pop_front() {
            Some(x) => x,
            None => return
        };

        // We need the read lock as long as we have handler.
        let read = self.handlers.read();

        let handler = unsafe {
            // Grab the internal HashMap from TypeMap and query for the data
            // with the TypeId we got from the queue.
            //
            // This should always contain a Handler of the appropriate type as we are
            // careful to only allow insertions of (EventKey<K, X>, Handler<X>) pairs.
            match read.data().find(&id) {
                Some(x) => x,
                // No handler for this event, move along.
                None => return
            // Downcast this handler to the fake type, which takes an opaque pointer
            // instead of the correct Box<Event<X>> because we cannot name X.
            }.downcast_ref_unchecked::<&Fn<(&(),), ()>>()
        };

        // Get the data as an opaque pointer. This is highly, highly unsafe but is fine
        // here because we carefully only inserted the correct event type for the correct
        // handler.
        //
        // This is necessary because we cannot name the real type behind this pointer here,
        // as that information was erased when we turned the Event into a Box<Any>.
        let event: &() = unsafe { event.downcast_ref_unchecked() };

        // Call the handler with an opaque pointer to the data. Since &T and Box<T> have
        // the same representation and &T and &() are also the same, the Handler's code
        // can actually treat the data as the type it expects, and all is good.
        handler.call((event,))
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

