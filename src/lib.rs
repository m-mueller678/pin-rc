#![no_std]
#![deny(unsafe_code)]

//! This crate provides reference counting pointers similar to `Rc` and `Arc`, but without heap allocation.
//! You are responsible for creating a `Pin{Arc|Rc}Storage`, which you can obtain `Pin{Arc|Rc}` pointers from.
//! The storage needs to be pinned, for example using [`pin`](core::pin::pin).
//!
//! ```rust
//! # use std::pin::pin;
//! # use pin_rc::{PinArc, PinArcStorage};
//! let storage = pin!(PinArcStorage::new(4));
//! let arc = storage.as_ref().create_handle();
//! println!("{arc:?}");
//! ```
//!
//! If the storage is dropped before all references to it are released, the program is aborted (even if you have set panics to unwind):
//! ```should_panic
//! # use std::pin::pin;
//! # use pin_rc::{PinArc,PinArcStorage};
//! fn escaping_handle() -> PinArc<u32> {
//!     let storage = pin!(PinArcStorage::new(4));
//!     storage.as_ref().create_handle()
//! }
//! escaping_handle();
//! ```

#[cfg(all(feature = "unsafe_disable_abort", not(debug_assertions)))]
const _: () = const {
    panic!("the feature unsafe_disable_abort should only be used for testing this crate. Enabling it makes the api unsound.")
};

use core::borrow::Borrow;
use core::cell::Cell;
use core::cmp::Ordering;
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use core::pin::Pin;
use core::sync::atomic::AtomicUsize;
use radium::Radium;

pub type PinRc<T> = PinRcGeneric<T, Cell<usize>>;
pub type PinRcStorage<T> = PinRcGenericStorage<T, Cell<usize>>;
pub type PinArc<T> = PinRcGeneric<T, AtomicUsize>;
pub type PinArcStorage<T> = PinRcGenericStorage<T, AtomicUsize>;

#[allow(unsafe_code)]
mod generic_rc;

use crate::generic_rc::Inner;
pub use generic_rc::{PinRcGeneric, PinRcGenericStorage};

impl<T, C: Radium<Item = usize>> Deref for PinRcGenericStorage<T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner().value()
    }
}

impl<T, C: Radium<Item = usize>> PinRcGenericStorage<T, C> {
    /// Get the number of handles currently referring to `self`.
    /// Beware of race conditions:
    /// Concurrent operations may change the count between
    /// the time you observe it and the time you act on the observation.
    pub fn ref_count(&self) -> usize {
        self.inner().count()
    }

    /// Create a handle referring to `self`.
    /// Note that this takes `Pin<&Self>`.
    /// If you have a `Pin<&mut Self>`, call `as_ref`:
    /// ```rust
    /// # use std::pin::pin;
    /// # use pin_rc::{PinArc, PinArcStorage};
    /// # let storage=pin!(PinArcStorage::new(4));
    /// let arc = storage.as_ref().create_handle();
    /// ```
    pub fn create_handle(self: Pin<&Self>) -> PinRcGeneric<T, C> {
        self.inner().create_handle()
    }
}

impl<T, C: Radium<Item = usize>> PinRcGeneric<T, C> {
    /// Get the number of handles currently referring to the same storage (including `self`).
    /// Beware of race conditions:
    /// Concurrent operations may change the count between
    /// the time you observe it and the time you act on the observation.
    pub fn ref_count(&self) -> usize {
        self.inner().count()
    }
}

impl<T, C: Radium<Item = usize>> Deref for PinRcGeneric<T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner().value()
    }
}

impl<T, C: Radium<Item = usize>> Clone for PinRcGeneric<T, C> {
    fn clone(&self) -> Self {
        self.inner().create_handle()
    }
}

macro_rules! impl_cmp_trait {
    ($Trait:ident{$($name:ident->$Ret:ty),*} for $For:ident) => {
        impl<T:$Trait,C:Radium<Item=usize>>  $Trait for $For<T,C>{
            $(
                #[inline]
                fn $name(&self, other: &Self)->$Ret{
                    <T as $Trait>::$name(&**self,&**other)
                }
            )*
        }
    };
}

impl_cmp_trait!(PartialEq{eq->bool} for PinRcGeneric);
impl_cmp_trait!(Eq{} for PinRcGeneric);
impl_cmp_trait!(PartialOrd{partial_cmp->Option<Ordering>,lt->bool,le->bool,gt->bool,ge->bool} for PinRcGeneric);
impl_cmp_trait!(Ord{cmp->Ordering} for PinRcGeneric);

impl_cmp_trait!(PartialEq{eq->bool} for PinRcGenericStorage);
impl_cmp_trait!(Eq{} for PinRcGenericStorage);
impl_cmp_trait!(PartialOrd{partial_cmp->Option<Ordering>,lt->bool,le->bool,gt->bool,ge->bool} for PinRcGenericStorage);
impl_cmp_trait!(Ord{cmp->Ordering} for PinRcGenericStorage);

macro_rules! impl_others {
    ($For:ident) => {
        impl<T: Hash, C: Radium<Item = usize>> Hash for $For<T, C> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                <T as Hash>::hash(&**self, state)
            }
        }

        impl<T, C: Radium<Item = usize>> Borrow<T> for $For<T, C> {
            fn borrow(&self) -> &T {
                self
            }
        }

        impl<T: Debug, C: Radium<Item = usize>> Debug for $For<T, C> {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                Debug::fmt(self.inner(), f)
            }
        }
    };
}

impl_others!(PinRcGeneric);
impl_others!(PinRcGenericStorage);

impl<T: Debug, C: Radium<Item = usize>> Debug for Inner<T, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("PinRcGeneric");
        s.field("ref_count", &self.count());
        s.field("value", self.value());
        s.finish()
    }
}
