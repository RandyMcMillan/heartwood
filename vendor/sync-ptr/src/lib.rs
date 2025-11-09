//! # sync-ptr
//! Sync & Send wrappers for raw pointer's in rust.
//! To use add `use sync_ptr::*;` to your file,
//! then you should be able to call `my_ptr.as_sync_const()` among others on any raw pointer
//! to obtain a wrapped version of your raw pointer that is Sync/Send.
//!
#![no_std]
#![deny(clippy::correctness)]
#![warn(
    clippy::perf,
    clippy::complexity,
    clippy::style,
    clippy::nursery,
    clippy::pedantic,
    clippy::clone_on_ref_ptr,
    clippy::decimal_literal_representation,
    clippy::float_cmp_const,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::unwrap_used,
    clippy::cargo_common_metadata,
    clippy::used_underscore_binding
)]
#![allow(clippy::inline_always)]
extern crate alloc;

use core::fmt::{Formatter, Pointer};
use core::ops::Deref;

/// Implement common traits for type `SelfType` by forwarding implementation
/// to underlying pointer.
///
/// Rust compiler cannot correctly auto-derive them because it's adding unnecessary
/// constraint equivalent to:
///
/// ```ignore
/// impl<T: Clone> Clone for SyncMutPtr<T> {...}
/// ```
///
/// It's not consistent with how these traits are implemented in built-in primitive pointers:
/// for example pointer can be cloned even if underlying type does not implement Clone, because
/// we are cloning pointer, not value it points to.
///
/// To make implementation of traits in this library consistent with implementation of same
/// traits on primitive pointers, we have to manually implement them.
macro_rules! trait_impl {
    ($SelfType:ident) => {
        impl<T> Clone for $SelfType<T> {
            #[inline(always)]
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<T> Copy for $SelfType<T> {}
        impl<T> Pointer for $SelfType<T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                core::fmt::Pointer::fmt(&self.0, f)
            }
        }

        impl<T> Eq for $SelfType<T> {}
        impl<T> PartialEq for $SelfType<T> {
            fn eq(&self, other: &Self) -> bool {
                PartialEq::eq(&self.0, &other.0)
            }
        }

        impl<T> PartialOrd for $SelfType<T> {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<T> Ord for $SelfType<T> {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                Ord::cmp(&self.0, &other.0)
            }
        }

        impl<T> core::fmt::Debug for $SelfType<T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!($SelfType)).field(&self.0).finish()
            }
        }

        impl<T> core::hash::Hash for $SelfType<T> {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                core::hash::Hash::hash(&self.0, state);
            }
        }
    };
}

///
/// Wrapped mutable raw pointer that is Send+Sync
///
#[repr(transparent)]
pub struct SyncMutPtr<T>(*mut T);

unsafe impl<T> Sync for SyncMutPtr<T> {}
unsafe impl<T> Send for SyncMutPtr<T> {}

trait_impl!(SyncMutPtr);

impl<T> SyncMutPtr<T> {
    ///
    /// Makes `ptr` Send+Sync
    ///
    /// # Safety
    /// The `ptr` parameter must be able to handle being sent and used in other threads concurrently,
    /// or special care must be taken when using the wrapped `ptr` to not use it
    /// in any way in other threads.
    ///
    #[inline(always)]
    #[must_use]
    pub const unsafe fn new(ptr: *mut T) -> Self {
        Self(ptr)
    }

    ///
    /// Makes a Send+Sync null ptr.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn null() -> Self {
        Self(core::ptr::null_mut())
    }

    ///
    /// Casts `ptr` to another data type while keeping it Send+Sync.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn cast<Y>(&self) -> SyncMutPtr<Y> {
        SyncMutPtr(self.0.cast())
    }

    ///
    /// Returns inner `ptr` which is then no longer Send+Sync.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> *mut T {
        self.0
    }

    ///
    /// Makes `ptr` immutable.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_sync_const(&self) -> SyncConstPtr<T> {
        SyncConstPtr(self.0)
    }

    ///
    /// Makes `ptr` immutable and no longer Sync.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_send_const(&self) -> SendConstPtr<T> {
        SendConstPtr(self.0)
    }

    ///
    /// This is equivalent to `.clone()` and does nothing.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_sync_mut(&self) -> Self {
        Self(self.0)
    }

    ///
    /// Makes `ptr` no longer Sync.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_send_mut(&self) -> SendMutPtr<T> {
        SendMutPtr(self.0)
    }
}

impl<T> Deref for SyncMutPtr<T> {
    type Target = *mut T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<SyncMutPtr<T>> for *mut T {
    #[inline(always)]
    fn from(val: SyncMutPtr<T>) -> Self {
        val.inner()
    }
}

impl<T> From<SyncMutPtr<T>> for *const T {
    #[inline(always)]
    fn from(val: SyncMutPtr<T>) -> Self {
        val.inner()
    }
}

///
/// Wrapped const raw pointer that is Send+Sync
///
#[repr(transparent)]
pub struct SyncConstPtr<T>(*const T);

unsafe impl<T> Sync for SyncConstPtr<T> {}
unsafe impl<T> Send for SyncConstPtr<T> {}

trait_impl!(SyncConstPtr);

impl<T> SyncConstPtr<T> {
    ///
    /// Makes `ptr` Send+Sync
    ///
    /// # Safety
    /// The `ptr` parameter must be able to handle being sent and used in other threads concurrently,
    /// or special care must be taken when using the wrapped `ptr` to not use it
    /// in any way in other threads.
    ///
    #[inline(always)]
    #[must_use]
    pub const unsafe fn new(ptr: *const T) -> Self {
        Self(ptr)
    }

    ///
    /// Makes a Send+Sync null ptr.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn null() -> Self {
        Self(core::ptr::null())
    }

    ///
    /// Casts `ptr` to another data type while keeping it Send+Sync.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn cast<Y>(&self) -> SyncConstPtr<Y> {
        SyncConstPtr(self.0.cast())
    }

    ///
    /// Returns inner `ptr` which is then no longer Send+Sync.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> *const T {
        self.0
    }

    ///
    /// This is equivalent to `.clone()` and does nothing.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_sync_const(&self) -> Self {
        Self(self.0)
    }

    ///
    /// Makes this `ptr` no longer Sync.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_send_const(&self) -> SendConstPtr<T> {
        SendConstPtr(self.0)
    }

    ///
    /// Makes this `ptr` mutable
    ///
    /// # Safety
    /// Writing to immutable data is UB.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_sync_mut(&self) -> SyncMutPtr<T> {
        SyncMutPtr(self.0.cast_mut())
    }

    ///
    /// Makes this `ptr` mutable and no longer Sync.
    ///
    /// # Safety
    /// Writing to immutable data is UB.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_send_mut(&self) -> SendMutPtr<T> {
        SendMutPtr(self.0.cast_mut())
    }
}

impl<T> Deref for SyncConstPtr<T> {
    type Target = *const T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<SyncConstPtr<T>> for *const T {
    #[inline(always)]
    fn from(val: SyncConstPtr<T>) -> Self {
        val.inner()
    }
}

///
/// Wrapped mutable raw pointer that is Send but not Sync
///
#[repr(transparent)]
pub struct SendMutPtr<T>(*mut T);

unsafe impl<T> Send for SendMutPtr<T> {}

trait_impl!(SendMutPtr);

impl<T> SendMutPtr<T> {
    ///
    /// Makes `ptr` Send
    ///
    /// # Safety
    /// The `ptr` parameter must be able to handle being sent to other threads
    /// or special care must be taken when using the wrapped `ptr` to not use it
    /// in any way in other threads.
    ///
    #[inline(always)]
    #[must_use]
    pub const unsafe fn new(ptr: *mut T) -> Self {
        Self(ptr)
    }
    ///
    /// Makes a Send null ptr.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn null() -> Self {
        Self(core::ptr::null_mut())
    }

    ///
    /// Casts `ptr` to another data type while keeping it Send.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn cast<Y>(&self) -> SendMutPtr<Y> {
        SendMutPtr(self.0.cast())
    }

    ///
    /// Returns inner `ptr` which is then no longer Send.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> *mut T {
        self.0
    }

    ///
    /// Makes this `ptr` Sync
    ///
    /// # Safety
    /// This `ptr` must be able to handle being accessed by multiple threads at the same time,
    /// or special care must be taken when using the wrapped `ptr` to not use it
    /// in any way in other threads.
    ///
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_sync_const(&self) -> SyncConstPtr<T> {
        SyncConstPtr(self.0)
    }

    ///
    /// Makes this `ptr` const.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_send_const(&self) -> SendConstPtr<T> {
        SendConstPtr(self.0)
    }

    ///
    /// Makes this `ptr` Sync
    ///
    /// # Safety
    /// This `ptr` must be able to handle being accessed by multiple threads at the same time,
    /// or special care must be taken when using the wrapped `ptr` to not use it
    /// in any way in other threads.
    ///
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_sync_mut(&self) -> SyncMutPtr<T> {
        SyncMutPtr(self.0)
    }

    ///
    /// This is equivalent to `.clone()` and does nothing.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_send_mut(&self) -> Self {
        Self(self.0)
    }
}

impl<T> Deref for SendMutPtr<T> {
    type Target = *mut T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<SendMutPtr<T>> for *mut T {
    #[inline(always)]
    fn from(val: SendMutPtr<T>) -> Self {
        val.inner()
    }
}

impl<T> From<SendMutPtr<T>> for *const T {
    #[inline(always)]
    fn from(val: SendMutPtr<T>) -> Self {
        val.inner()
    }
}

///
/// Wrapped const raw pointer that is Send but not Sync
///
#[repr(transparent)]
pub struct SendConstPtr<T>(*const T);

unsafe impl<T> Send for SendConstPtr<T> {}

trait_impl!(SendConstPtr);

impl<T> SendConstPtr<T> {
    ///
    /// Makes `ptr` Send
    ///
    /// # Safety
    /// The `ptr` parameter must be able to handle being sent to other threads
    /// or special care must be taken when using the wrapped `ptr` to not use it
    /// in any way in other threads.
    ///
    #[inline(always)]
    #[must_use]
    pub const unsafe fn new(ptr: *const T) -> Self {
        Self(ptr)
    }

    ///
    /// Makes a Send null ptr.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn null() -> Self {
        Self(core::ptr::null())
    }

    ///
    /// Casts `ptr` to another data type while keeping it Send.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn cast<Y>(&self) -> SendConstPtr<Y> {
        SendConstPtr(self.0.cast())
    }

    ///
    /// Returns inner `ptr` which is then no longer Send.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> *const T {
        self.0
    }

    ///
    /// Makes this `ptr` Sync
    ///
    /// # Safety
    /// This `ptr` must be able to handle being accessed by multiple threads at the same time,
    /// or special care must be taken when using the wrapped `ptr` to not use it
    /// in any way in other threads.
    ///
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_sync_const(&self) -> SyncConstPtr<T> {
        SyncConstPtr(self.0)
    }

    ///
    /// This is equivalent to `.clone()` and does nothing.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_send_const(&self) -> Self {
        Self(self.0)
    }

    ///
    /// Makes this `ptr` Sync
    ///
    /// # Safety
    /// This `ptr` must be able to handle being accessed by multiple threads at the same time,
    /// or special care must be taken when using the wrapped `ptr` to not use it
    /// in any way in other threads.
    ///
    /// `ptr` is also marked as mutable. Writing to immutable data is usually UB.
    ///
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_sync_mut(&self) -> SyncMutPtr<T> {
        SyncMutPtr(self.0.cast_mut())
    }

    ///
    /// Makes this `ptr` mutable
    ///
    /// # Safety
    /// Writing to immutable data is UB.
    ///
    #[inline(always)]
    #[must_use]
    pub const fn as_send_mut(&self) -> SendMutPtr<T> {
        SendMutPtr(self.0.cast_mut())
    }
}

impl<T> Deref for SendConstPtr<T> {
    type Target = *const T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<SendConstPtr<T>> for *const T {
    #[inline(always)]
    fn from(val: SendConstPtr<T>) -> *const T {
        val.inner()
    }
}

pub trait FromConstPtr<T>: Sized {
    ///
    /// Makes `self` immutable and Send+Sync
    ///
    /// # Safety
    /// `self` must be able to handle being sent to and used concurrently by other threads,
    /// or special care must be taken when using the wrapped `self` to not use it
    /// in any way in other threads.
    ///
    unsafe fn as_sync_const(&self) -> SyncConstPtr<T>;

    ///
    /// Makes `self` immutable and Send
    ///
    /// # Safety
    /// `self` must be able to handle being sent to other threads
    /// or special care must be taken when using the wrapped `self` to not use it
    /// in any way in other threads.
    ///
    unsafe fn as_send_const(&self) -> SendConstPtr<T>;
}

pub trait FromMutPtr<T>: FromConstPtr<T> {
    ///
    /// Makes `self` Send+Sync
    ///
    /// # Safety
    /// `self` must be able to handle being sent to and used concurrently by other threads,
    /// or special care must be taken when using the wrapped `self` to not use it
    /// in any way in other threads.
    ///
    unsafe fn as_sync_mut(&self) -> SyncMutPtr<T>;

    ///
    /// Makes `self` Send
    ///
    /// # Safety
    /// `self` must be able to handle being sent to other threads
    /// or special care must be taken when using the wrapped `self` to not use it
    /// in any way in other threads.
    ///
    unsafe fn as_send_mut(&self) -> SendMutPtr<T>;
}

impl<T> FromConstPtr<T> for *const T {
    #[inline(always)]
    unsafe fn as_sync_const(&self) -> SyncConstPtr<T> {
        SyncConstPtr(self.cast())
    }

    #[inline(always)]
    unsafe fn as_send_const(&self) -> SendConstPtr<T> {
        SendConstPtr(self.cast())
    }
}

impl<T> FromConstPtr<T> for *mut T {
    #[inline(always)]
    unsafe fn as_sync_const(&self) -> SyncConstPtr<T> {
        SyncConstPtr(self.cast())
    }

    #[inline(always)]
    unsafe fn as_send_const(&self) -> SendConstPtr<T> {
        SendConstPtr(self.cast())
    }
}

impl<T> FromMutPtr<T> for *mut T {
    #[inline(always)]
    unsafe fn as_sync_mut(&self) -> SyncMutPtr<T> {
        SyncMutPtr(self.cast())
    }

    #[inline(always)]
    unsafe fn as_send_mut(&self) -> SendMutPtr<T> {
        SendMutPtr(self.cast())
    }
}
