//! A versatile and easy to use defer statement for Rust. Similar to Go's or Zig's defer.
//!
//! This crate is compatible uses `no_std`.
//! The default features use `alloc`
//! To disable alloc set default-features to false in cargo.toml.
//!
//! This crates provides 6 macros for different use cases of deferment:
//! 1. `defer!` simple deferment. Will execute when current scope ends.
//!
//! 2. `defer_move!` same as `defer!` but moves local variables into the closure.
//!
//! 3. `defer_guard!` Returns a guard that causes execution when its scope ends.
//!     - Execution can be canceled or preempted.
//!
//! 4. `defer_move_guard!` Same as `defer_guard!` but moves local variables into the closure.
//!
//! 5. `defer_arc!` Returns a reference counted guard than can be shared with other threads.
//!     - Execution can be canceled or preempted.
//!     - Closure must be `Send`
//!     - Target must support Arc & AtomicBool.
//!     - Target must support alloc
//!     - can be disabled with `default-features=false` in Cargo.toml
//!
//! 6. `defer_move_arc!` Same as `defer_arc!` but moves local variables into the closure.
//!     - All used local variables must be `Send`.
//!
//! # Usage
//!
//! Add the dependency in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! defer-heavy = "0.1.0"
//! ```
//!
//! # Order of execution
//! Rust guarantees that the order in which the closures are dropped
//! (and therefore executed) are in reverse order of creation.
//! This means the last `defer!` in the scope executes first.
//!
//!

#![no_std]

#[cfg(target_has_atomic = "8")]
#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "mt")]
mod mt {
    extern crate alloc;
    use crate::DeferGuard;
    use alloc::sync::Arc;
    use core::sync::atomic::AtomicBool;
    use core::sync::atomic::Ordering::SeqCst;

    #[doc(hidden)]
    #[derive(Debug, Clone)]
    pub struct ArcDeferGuard<F: FnOnce() + Send>(Arc<ArcDeferGuardInner<F>>);

    #[doc(hidden)]
    impl<F: FnOnce() + Send> ArcDeferGuard<F> {
        #[inline(always)]
        #[must_use]
        pub fn new(func: F) -> Self {
            Self(Arc::new(ArcDeferGuardInner(
                AtomicBool::new(false),
                Some(func),
            )))
        }

        #[inline(always)]
        pub(crate) fn new_opt(func: Option<F>) -> Self {
            Self(Arc::new(ArcDeferGuardInner(
                AtomicBool::new(func.is_none()),
                func,
            )))
        }

        ///
        /// Utility function to ensure ownership is transferred to a thread/closure.
        ///
        #[inline(always)]
        #[must_use]
        pub fn own(self) -> Self {
            self
        }

        ///
        /// Downgrade the guard to a non reference counted guard.
        ///
        /// # Returns
        /// * Ok: this was the only reference to the guard. The guard was downgraded.
        /// * Err: there is still more than 1 reference to the guard.
        ///
        #[inline(always)]
        pub fn try_downgrade(self) -> Result<DeferGuard<F>, Self> {
            let mut inner = Arc::try_unwrap(self.0).map_err(|a| ArcDeferGuard(a))?;
            if !inner.0.load(SeqCst) {
                return Ok(DeferGuard(inner.1.take()));
            }

            Ok(DeferGuard(None))
        }

        ///
        /// Try to call the closure.
        /// This will succeed if no other references to it exist.
        /// a return value of OK always indicates that the closure was dropped.
        ///
        /// # Returns
        /// * Ok(true): closure was called
        /// * Ok(false): closure was not called because it is already canceled.
        /// * Err: there is still more than 1 reference to the guard.
        ///
        ///
        pub fn try_destroy(self) -> Result<bool, Self> {
            let inner = Arc::try_unwrap(self.0).map_err(|a| ArcDeferGuard(a))?;
            //DROP inner which calls the closure if inner.0 (canceled flag) is not true.
            Ok(!inner.0.load(SeqCst))
        }

        ///
        /// Will cancel running the closure, so it cannot be called anymore.
        /// The closure is dropped once no thread has a reference to it anymore,
        /// however it is guaranteed to not get called anymore.
        ///
        #[inline(always)]
        pub fn cancel(self) {
            self.0 .0.store(true, SeqCst)
        }

        ///
        /// Will cancel running the closure, so it cannot be called anymore.
        /// The closure is dropped once no thread has a reference to it anymore,
        /// however it is guaranteed to not get called anymore.
        ///
        #[inline(always)]
        pub fn cancel_ref(&self) {
            self.0 .0.store(true, SeqCst)
        }
    }

    impl<T: FnOnce() + Send> TryFrom<ArcDeferGuard<T>> for DeferGuard<T> {
        type Error = ArcDeferGuard<T>;

        fn try_from(value: ArcDeferGuard<T>) -> Result<Self, Self::Error> {
            value.try_downgrade()
        }
    }

    impl<T: FnOnce() + Send> From<DeferGuard<T>> for ArcDeferGuard<T> {
        fn from(value: DeferGuard<T>) -> Self {
            value.upgrade()
        }
    }

    #[derive(Debug)]
    struct ArcDeferGuardInner<F: FnOnce() + Send>(AtomicBool, Option<F>);

    impl<F: FnOnce() + Send> Drop for ArcDeferGuardInner<F> {
        fn drop(&mut self) {
            if !self.0.load(SeqCst) {
                if let Some(f) = self.1.take() {
                    f()
                }
            }
        }
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct DeferGuard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> DeferGuard<F> {
    #[inline(always)]
    #[must_use]
    pub fn new(func: F) -> Self {
        Self(Some(func))
    }

    ///
    /// Upgrade the guard to a reference counted one.
    /// This function is only available if the closure is `Send`
    ///
    /// # Returns
    /// The reference counted guard.
    ///
    #[cfg(target_has_atomic = "8")]
    #[cfg(target_has_atomic = "ptr")]
    #[cfg(feature = "mt")]
    pub fn upgrade(mut self) -> ArcDeferGuard<F>
    where
        F: FnOnce() + Send,
    {
        ArcDeferGuard::new_opt(self.0.take())
    }

    ///
    /// Will call the closure now.
    ///
    /// # Returns
    /// * true: closure was called.
    /// * false: closure was not called because `cancel_ref` or `destroy_ref` was called previously.
    ///
    #[inline(always)]
    pub fn destroy(mut self) -> bool {
        self.0.take().map(|f| f()).is_some()
    }

    ///
    /// Will call the closure now.
    /// This drops the closure.
    ///
    /// # Returns
    /// * true: closure was called.
    /// * false: closure was not called because `cancel_ref` or `destroy_ref` was called previously.
    ///
    #[inline(always)]
    pub fn destroy_ref(&mut self) -> bool {
        self.0.take().map(|f| f()).is_some()
    }

    ///
    /// Will cancel running the closure, so it cannot be called anymore.
    ///
    /// # Returns
    /// * true: closure was dropped and will not be called anymore.
    /// * false: closure was already dropped previously because `cancel_ref` or `destroy_ref` was called previously.
    ///
    #[inline(always)]
    pub fn cancel(mut self) -> bool {
        self.0.take().is_some()
    }

    ///
    /// Will cancel the closure, so it cannot be called anymore.
    /// This drops the closure.
    ///
    /// # Returns
    /// * true: closure was dropped and will not be called anymore.
    /// * false: closure was already dropped previously because `cancel_ref` or `destroy_ref` was called previously.
    ///
    #[inline(always)]
    pub fn cancel_ref(&mut self) -> bool {
        self.0.take().is_some()
    }
}

impl<F: FnOnce()> Drop for DeferGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f()
        }
    }
}

/// Executes a block of code when the surrounding scope ends.
///
/// # Examples
///
/// Simplest example:
///
/// ```rust
/// use defer_heavy::defer;
///
/// fn test() {
///     defer! { println!("Second"); }
///     println!("First");
/// }
/// ```
///
/// Multiple statements:
///
/// ```rust
/// use defer_heavy::defer;
///
/// fn test() {
///     defer! {
///         println!("Second");
///         println!("Third");
///     }
///     println!("First");
/// }
/// ```
///
/// In Go, the `defer` code runs when the function exits. In this Rust
/// implementation, however, the code runs when the surrounding scope ends –
/// this makes it possible to use `defer` inside loops:
///
/// ```rust
/// use defer_heavy::defer;
///
/// fn test() {
///     defer! { println!("End"); }
///     println!("Before");
///
///     for i in 0..2 {
///         defer! { println!("Defer {}", i); }
///         println!("Loop {}", i);
///     }
///
///     println!("After");
/// }
/// ```
#[macro_export]
macro_rules! defer {
	( $($tt:tt)* ) => {
		let _deferred = $crate::DeferGuard::new(|| { $($tt)* });
	};
}

/// Executes a block of code when the surrounding scope ends.
/// This macro moves all captured variables.
///
/// # Examples
///
/// ```rust
/// use defer_heavy::defer_move;
///
/// fn test() {
///     let n = 1;
///     defer_move! { println!("Second n={}", n); }
///     println!("First");
/// }
/// ```
#[macro_export]
macro_rules! defer_move {
	( $($tt:tt)* ) => {
		let _deferred = $crate::DeferGuard::new(move || { $($tt)* });
	};
}

/// Executes a block of code when the surrounding scope ends.
///
/// The macro returns a guard that defines the scope of the deferment.
/// The guard can be used to immediately execute the deferred closure or cancel it and
/// prevent execution of the closure altogether
///
/// # Examples
///
/// ```rust
/// use defer_heavy::defer_guard;
///
/// fn test() {
///
///     let defer1 = defer_guard! { unreachable!("Wont be executed"); };
///     let defer2 = defer_guard! { println!("Second"); };
///
///     println!("First");
///     defer1.cancel();
/// }
/// ```
///
/// ```rust
/// use defer_heavy::defer_guard;
///
/// fn test() {
///
///     let defer1 = defer_guard! { unreachable!("Wont be executed"); };
///     let defer2 = defer_guard! { println!("Fourth"); };
///     let defer3 = defer_guard! { println!("Second"); };
///
///     println!("First");
///     defer3.destroy();
///     println!("Third");
///     defer1.cancel();
/// }
/// ```
///
#[macro_export]
macro_rules! defer_guard {
	( $($tt:tt)* ) => {
		$crate::DeferGuard::new(|| { $($tt)* });
	};
}

/// Executes a block of code when the surrounding scope ends.
///
/// The macro returns a guard that defines the scope of the deferment.
/// The guard can be used to immediately execute the deferred closure or cancel it and
/// prevent execution of the closure altogether
///
/// # Examples
///
/// ```rust
/// use defer_heavy::defer_move_guard;
///
/// fn test() {
///     let n = 1;
///     let defer1 = defer_move_guard! { unreachable!("Wont be executed {}", n); };
///     let n = 2;
///     let defer2 = defer_move_guard! { println!("Second {}", n); };
///
///     println!("First");
///     defer1.cancel();
/// }
/// ```
/// Prints:
/// ```text
/// First
/// Second 2
/// ```
///
/// ```rust
/// use defer_heavy::defer_move_guard;
///
/// fn test() {
///
///     let n = 1;
///     let defer1 = defer_move_guard! { unreachable!("Wont be executed {}", n); };
///     let n = 2;
///     let defer2 = defer_move_guard! { println!("Fourth {}", n); };
///     let n = 3;
///     let defer3 = defer_move_guard! { println!("Second {}", n); };
///
///     println!("First");
///     defer3.destroy(); //Same as drop(defer3);
///     println!("Third");
///     defer1.cancel();
/// }
/// ```
/// Prints:
/// ```text
/// First
/// Second 3
/// Third
/// Fourth 2
/// ```
///
/// ```rust
///
#[macro_export]
macro_rules! defer_move_guard {
	( $($tt:tt)* ) => {
		$crate::DeferGuard::new(move || { $($tt)* });
	};
}

#[cfg(target_has_atomic = "8")]
#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "mt")]
pub use mt::ArcDeferGuard;

/// Executes a block of code when the surrounding scope ends.
///
/// The macro returns a guard that defines the scope of the deferment.
/// The guard can be shared with other threads, and it will only execute the block of code when
/// no more references to the guard exist.
///
/// The code closure must be 'Send'.
///
///
/// # Examples
/// ```rust
/// use std::thread;
/// use std::time::Duration;
/// use defer_heavy::defer_arc;
///
/// pub fn test() {
///     let deferred = defer_arc! { println!("Executed in {:?}", thread::current().id());};
///     println!("Main thread {:?}", thread::current().id());
///     let th;
///     {
///         let deferred = deferred.clone();
///         th = thread::spawn(move ||{
///             println!("Spawned thread {:?}", thread::current().id());
///             let _deferred = deferred.own();
///             thread::sleep(Duration::from_millis(2000)); //SIMULATE work
///        });
///    }
///    thread::sleep(Duration::from_millis(2000)); //SIMULATE WORK
///    drop(deferred);
///    th.join().unwrap();
///  }
/// ```
/// Prints:
/// ```text
/// Main thread Thread(1)
/// Spawned thread Thread(2)
/// "Executed in Thread(1)" or "Executed in Thread(2)"
/// ```
///
#[cfg(target_has_atomic = "8")]
#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "mt")]
#[macro_export]
macro_rules! defer_arc {
	( $($tt:tt)* ) => {
		$crate::ArcDeferGuard::new(|| { $($tt)* });
	};
}

/// Executes a block of code when the surrounding scope ends.
///
/// The macro returns a guard that defines the scope of the deferment.
/// The guard can be shared with other threads, and it will only execute the block of code when
/// no more references to the guard exist.
///
/// The code closure must be 'Send'.
/// The closure moves all used variables. Each used variable must be 'Send' or the closure won't be 'Send'.
///
/// # Examples
/// ```rust
/// use std::thread;
/// use std::time::Duration;
/// use defer_heavy::defer_move_arc;
///
/// pub fn test() {
///     let v = 1;
///     let deferred = defer_move_arc! { println!("Executed {} in {:?}", v, thread::current().id());};
///     println!("Main thread {:?}", thread::current().id());
///     let th;
///     {
///         let deferred = deferred.clone();
///         th = thread::spawn(move ||{
///             println!("Spawned thread {:?}", thread::current().id());
///             let _deferred = deferred.own();
///             thread::sleep(Duration::from_millis(2000)); //SIMULATE work
///        });
///    }
///    thread::sleep(Duration::from_millis(2000)); //SIMULATE WORK
///    drop(deferred);
///    th.join().unwrap();
///  }
/// ```
/// Prints:
/// ```text
/// Main thread Thread(1)
/// Spawned thread Thread(2)
/// "Executed in Thread(1)" or "Executed in Thread(2)"
/// ```
///
#[cfg(target_has_atomic = "8")]
#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "mt")]
#[macro_export]
macro_rules! defer_move_arc {
	( $($tt:tt)* ) => {
		$crate::ArcDeferGuard::new(move || { $($tt)* });
	};
}
