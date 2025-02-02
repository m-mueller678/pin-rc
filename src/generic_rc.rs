use core::cell::UnsafeCell;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use radium::Radium;

const MAX_REFCOUNT: usize = usize::MAX / 2;

pub struct PinRcGenericStorage<T, C: Radium<Item = usize>> {
    inner: UnsafeCell<Inner<T, C>>,
    _p: PhantomPinned,
}

pub(crate) struct Inner<T, C> {
    count: C,
    value: T,
}

impl<T, C: Radium<Item = usize>> Inner<T, C> {
    pub(crate) fn count(&self) -> usize {
        self.count.load(Relaxed)
    }

    pub(crate) fn value(&self) -> &T {
        &self.value
    }

    pub(crate) fn create_handle(&self) -> PinRcGeneric<T, C> {
        let old_count = self.count.fetch_add(1, Relaxed);
        if old_count > MAX_REFCOUNT {
            abort()
        }
        PinRcGeneric(self)
    }
}

fn abort() -> ! {
    if cfg!(feature = "unsafe_disable_abort") {
        panic!()
    } else {
        extern "C" fn force_abort() -> ! {
            // A panic hook might run here.
            // As this is called from the destructor, the memory of the storage has not been reused yet,
            // so it would be fine for the hook to access the contained value.
            panic!()
        }
        force_abort()
    }
}

impl<T, C: Radium<Item = usize>> Drop for PinRcGenericStorage<T, C> {
    fn drop(&mut self) {
        if self.inner().count.load(Acquire) != 0 {
            abort()
        }
    }
}

impl<T, C: Radium<Item = usize>> PinRcGenericStorage<T, C> {
    /// Create a new storage containing the provided value.
    pub fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(Inner {
                value,
                count: C::new(0),
            }),
            _p: Default::default(),
        }
    }

    /// Get a mutable reference to the contents if there are no handles referring to `self`.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Option<Pin<&mut T>> {
        if self.inner().count.load(Acquire) == 0 {
            Some(unsafe { Pin::new_unchecked(&mut (*self.inner.get()).value) })
        } else {
            None
        }
    }

    pub(crate) fn inner(&self) -> &Inner<T, C> {
        unsafe { &*self.inner.get() }
    }
}

pub struct PinRcGeneric<T, C: Radium<Item = usize>>(*const Inner<T, C>);

impl<T, C: Radium<Item = usize>> PinRcGeneric<T, C> {
    pub(crate) fn inner(&self) -> &Inner<T, C> {
        unsafe { &*self.0 }
    }
}

impl<T, C: Radium<Item = usize>> Drop for PinRcGeneric<T, C> {
    fn drop(&mut self) {
        let c = self.inner().count.fetch_sub(1, Release);
        debug_assert!(c > 0);
    }
}
