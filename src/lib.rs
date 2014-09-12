#![feature(macro_rules)]
#![license = "MIT"]
//#![deny(missing_doc)]
#![deny(warnings)]

//! Crate comment goes here

local_data_key!(LocalEventQueue: Arc<EventQueue>)

macro_rules! queue(
    () => {{
        LocalEventQueue.get().expect("an Event Queue to be in TLS")
    }}
)

pub struct Event<T: Send> {
    data: T
}

impl<T: Send> Event<T> {
    pub fn new(val: T) -> Event<T> {
        Event { data: val }
    }

    pub fn trigger<K: Assoc<T>>(self) {
        (queue!()).queue::<K, T>(self)
    }
}

pub fn on<K: Assoc<X>, X: Send>(handler: Handler<X>) {
    (queue!()).on::<K, X>(handler)
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
    // unchecked downcast of the Handler to a Box<Fn<(Box<()>,), ()>>
    // and downcast the Event to Event<()>, access the data, box it,
    // and call the Handler with the data.
    queue: Mutex<RingBuf<(TypeId, Box<Any + Send>)>>,
    handlers: RWLock<TypeMap>
}

impl EventQueue {
    pub fn new() -> Arc<EventQueue> {
        let this = Arc::new(EventQueue {
            queue: Mutex::new(RingBuf::new()),
            handlers: RWLock::new(TypeMap::new())
        });
        LocalEventQueue.replace(Some(this.clone()));
        this
    }

    fn queue<K: Assoc<X>, X: Send>(&self, event: Event<X>) {
        self.queue.lock().push((TypeId::of::<EventKey<K, X>>(), box event as Box<Any + Send>));
    }

    fn on<K: Assoc<X>, X: Send>(&self, handler: Handler<X>) {
        self.handlers.write().insert::<EventKey<K, X>, Handler<X>>(handler);
    }
}

