#![no_std]
#![deny(unsafe_code)]

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

macro_rules! impl_hash_borrow {
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
    };
}

impl_hash_borrow!(PinRcGeneric);
impl_hash_borrow!(PinRcGenericStorage);

macro_rules! impl_other_trait {
    ($Trait:ident{$($method:ident($($arg:ident:$Arg:ty),*)->$Ret:ty),*} for $For:ident) => {
        impl<T:$Trait,C:Radium<Item=usize>>  $Trait for $For<T,C>{
            $(
                #[inline]
                fn $method(&self, $($arg:$Arg),*)->$Ret{
                    <T as $Trait>::$method(&**self,$($arg),*)
                }
            )*
        }
    };
}

impl_other_trait! {Debug{fmt(f: &mut Formatter<'_>)->core::fmt::Result} for PinRcGeneric}
impl_other_trait! {Debug{fmt(f: &mut Formatter<'_>)->core::fmt::Result} for PinRcGenericStorage}
