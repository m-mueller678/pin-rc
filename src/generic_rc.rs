use core::cell::{Cell, UnsafeCell};
use core::marker::{PhantomData, PhantomPinned};
use core::pin::Pin;
use core::ptr::NonNull;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use radium::Radium;

const MAX_REFCOUNT: usize = usize::MAX / 2;

/// The common implementation shared by [PinRcStorage](crate::PinRcStorage) and [PinArcStorage](crate::PinArcStorage).
pub struct PinRcGenericStorage<T, C: Radium<Item = usize>> {
    inner: UnsafeCell<Inner<T, C>>,
    _p: PhantomPinned,
    _ps: PhantomData<*const u32>, // prevent Send and Sync
}

pub(crate) struct Inner<T, C> {
    count: C,
    value: T,
}

impl<T, C: Radium<Item = usize>> Inner<T, C> {
    pub(crate) fn count(&self) -> usize {
        self.count.load(Relaxed)
    }

    pub(crate) fn value_pin(self: Pin<&Self>) -> Pin<&T> {
        unsafe { Pin::new_unchecked(&self.get_ref().value) }
    }

    pub(crate) fn value_unpin(&self) -> &T {
        &self.value
    }

    pub(crate) fn create_handle(self: Pin<&Self>) -> PinRcGeneric<T, C> {
        let old_count = self.count.fetch_add(1, Relaxed);
        if old_count > MAX_REFCOUNT {
            abort()
        }
        PinRcGeneric(NonNull::from(self.get_ref()))
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
        if self.inner_unpin().count.load(Acquire) != 0 {
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
            _ps: PhantomData,
        }
    }

    /// Get a mutable reference to the contents if there are no handles referring to `self`.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Option<Pin<&mut T>> {
        if self.as_ref().inner_unpin().count.load(Acquire) == 0 {
            Some(unsafe { Pin::new_unchecked(&mut (*self.inner.get()).value) })
        } else {
            None
        }
    }

    pub(crate) fn inner_pin(self: Pin<&Self>) -> Pin<&Inner<T, C>> {
        unsafe { Pin::new_unchecked(&*self.inner.get()) }
    }

    pub(crate) fn inner_unpin(&self) -> &Inner<T, C> {
        unsafe { &*self.inner.get() }
    }
}

/// The common implementation shared by [PinRc](crate::PinRc) and [PinArc](crate::PinArc).
pub struct PinRcGeneric<T, C: Radium<Item = usize>>(NonNull<Inner<T, C>>);

impl<T, C: Radium<Item = usize>> PinRcGeneric<T, C> {
    pub(crate) fn inner_pin(&self) -> Pin<&Inner<T, C>> {
        unsafe { Pin::new_unchecked(self.0.as_ref()) }
    }

    pub(crate) fn inner_unpin(&self) -> &Inner<T, C> {
        self.inner_pin().get_ref()
    }
}

impl<T, C: Radium<Item = usize>> Drop for PinRcGeneric<T, C> {
    fn drop(&mut self) {
        let c = self.inner_unpin().count.fetch_sub(1, Release);
        debug_assert!(c > 0);
    }
}

pub type PinRc<T> = PinRcGeneric<T, Cell<usize>>;
pub type PinRcStorage<T> = PinRcGenericStorage<T, Cell<usize>>;
pub type PinArc<T> = PinRcGeneric<T, AtomicUsize>;
pub type PinArcStorage<T> = PinRcGenericStorage<T, AtomicUsize>;

unsafe impl<T> Sync for PinArc<T> where T: Sync {}
unsafe impl<T> Sync for PinArcStorage<T> where T: Sync {}
unsafe impl<T> Send for PinArc<T> where T: Sync {}
unsafe impl<T> Send for PinArcStorage<T> where T: Send + Sync {}
