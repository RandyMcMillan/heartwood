# sync-ptr
Sync & Send wrappers for raw pointer's in rust.

This crate uses `#![no_std]` and can be used in projects that do not use the rust standard library.

### Intended use
This crate is intended for handles or pointers to data behind an FFI boundary 
that is known to be Send or Sync. 

Example where this is most likely the case:
- shared memory from mmap() or MapViewOfFile()
- global JNI handles
- ffi mutex handles etc
- ...

I would like to mention that this crate does not magically make any 
C/FFI pointers into arbitrary data Send or Sync safely. 
Do not use it on pointers/handles that cannot handle being Send or Sync 
or you may cause UB on the C/FFI side.

This crate is not intended for usage with pointers to rust data.
While it can be used like this, one has to be VERY careful to avoid UB.
Having raw pointers shared across threads certainly 
does not help with handling lifetimes properly.

The only exception to both of this are rare cases where the pointers/handles themselves
are Sent to other threads and those do not perform any operation on the pointers/handles.
For example a hypothetical multithreaded sorting algorithm that sorts an array of pointers 
without accessing or using the pointers/handles in any way and only sends the result 
back to the original thread where the pointers/handles are then used should always be safe.

### Example

```rust
use std::ffi::c_void;
use std::ptr::null_mut;
use sync_ptr::*;

struct RustControlStructureThatIsNowSend {
    some_handle: SendConstPtr<c_void>,
    some_rust_data: u64
}

#[test]
fn example() {
    //Some handle or pointer obtained goes here.
    let handle: *mut c_void = null_mut();
    let data: u64 = 123u64;

    let rcs = RustControlStructureThatIsNowSend {
        some_handle: unsafe {handle.as_send_const()},
        some_rust_data: data,
    };

    unsafe {
        //Every *const T and *mut T has these fn's now.
        let _sync_const: SyncConstPtr<c_void> = handle.as_sync_const(); //This is unsafe.
        let _send_const: SendConstPtr<c_void> = handle.as_send_const(); //This is unsafe.

        //Every *mut T has these fn's now.
        let _sync_mut: SyncMutPtr<c_void> = handle.as_sync_mut(); //This is unsafe.
        let _send_mut: SendMutPtr<c_void> = handle.as_send_mut(); //This is unsafe.

        //The other Ptr types have the same constructors too.
        let _send_const_null: SendMutPtr<c_void> = SendMutPtr::null(); //This is safe.
        let _send_const_new: SendMutPtr<c_void> = SendMutPtr::new(null_mut()); //This is unsafe.
    }

    std::thread::spawn(move || {
        assert!(rcs.some_handle.is_null()); //Use boxed
        let _unwrapped: *const c_void = rcs.some_handle.inner(); //unwrap if you want
        let _unwrapped2: *const c_void = rcs.some_handle.into(); //Into<*const T> is also implemented. (*mut T too when applicable)
        let casted: SendConstPtr<usize> = rcs.some_handle.cast::<usize>(); //Cast if you want.
        unsafe {
            if !casted.is_null() { //In this example this is obviously always null...
                casted.read_volatile(); //Read if you want
            }
        }

        assert_eq!(rcs.some_rust_data, 123u64)
    }).join().unwrap();
}
```

#### Why not just make RustControlStructureThatIsNowSend implement Send directly?
This is prone to error as once a struct is "unsafe impl Sync" for example it will be Sync no matter what
struct members get added later. If the initial reason for that unsafe impl was a raw pointer, then 
the compiler has no opportunity to inform the Human that adding a RefCell to such a struct is maybe not a good idea.

In addition, there are sometimes cases where one only needs to 
send a single pointer and writing an unsafe impl wrapper struct
everytime is annoying.