use core::nonzero::NonZero;
use std::cell::RefCell;

pub struct MutScoped<T> {
    inner: RefCell<NonZero<*mut T>>
}

impl<T: 'static> MutScoped<T> {
    pub fn new(reff: &mut T) -> MutScoped<T> {
        MutScoped { inner: RefCell::new(unsafe { NonZero::new(reff as *mut _) }) }
    }

    pub unsafe fn borrow_mut<F: FnOnce(&mut T) -> R, R>(&self, cb: F) -> R {
        cb(&mut ***self.inner.borrow_mut())
    }
}

