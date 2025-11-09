extern crate alloc;
extern crate std;

use alloc::{format, vec};
use core::ffi::c_void;
use core::ptr::null_mut;
use sync_ptr::*;

#[test]
pub fn test_debug() {
    let n = unsafe { null_mut::<c_void>().as_sync_mut() };
    assert_eq!(format!("{:?}", n), "SyncMutPtr(0x0)");
    let n = unsafe { null_mut::<c_void>().as_sync_const() };
    assert_eq!(format!("{:?}", n), "SyncConstPtr(0x0)");
    let n = unsafe { null_mut::<c_void>().as_send_const() };
    assert_eq!(format!("{:?}", n), "SendConstPtr(0x0)");
    let n = unsafe { null_mut::<c_void>().as_send_mut() };
    assert_eq!(format!("{:?}", n), "SendMutPtr(0x0)");

    #[cfg(target_pointer_width = "64")]
    {
        let n = unsafe { null_mut::<c_void>().as_sync_mut() };
        assert_eq!(
            format!("{:#?}", n),
            "SyncMutPtr(\n    0x0000000000000000,\n)"
        );
        let n = unsafe { null_mut::<c_void>().as_sync_const() };
        assert_eq!(
            format!("{:#?}", n),
            "SyncConstPtr(\n    0x0000000000000000,\n)"
        );
        let n = unsafe { null_mut::<c_void>().as_send_const() };
        assert_eq!(
            format!("{:#?}", n),
            "SendConstPtr(\n    0x0000000000000000,\n)"
        );
        let n = unsafe { null_mut::<c_void>().as_send_mut() };
        assert_eq!(
            format!("{:#?}", n),
            "SendMutPtr(\n    0x0000000000000000,\n)"
        );
    }

    #[cfg(target_pointer_width = "32")]
    {
        let n = unsafe { null_mut::<c_void>().as_sync_mut() };
        assert_eq!(format!("{:#?}", n), "SyncMutPtr(\n    0x00000000,\n)");
        let n = unsafe { null_mut::<c_void>().as_sync_const() };
        assert_eq!(format!("{:#?}", n), "SyncConstPtr(\n    0x00000000,\n)");
        let n = unsafe { null_mut::<c_void>().as_send_const() };
        assert_eq!(format!("{:#?}", n), "SendConstPtr(\n    0x00000000,\n)");
        let n = unsafe { null_mut::<c_void>().as_send_mut() };
        assert_eq!(format!("{:#?}", n), "SendMutPtr(\n    0x00000000,\n)");
    }
}

#[test]
pub fn test_cmp() {
    unsafe {
        let mut data = vec![0; 4096];
        let n = data.as_mut_ptr().as_sync_mut();
        let x = n.add(5).as_sync_mut();
        assert!(x > n);
        assert!(n < x);
        assert_ne!(n, x);
        assert_eq!(n, n);
        assert_eq!(x, x);
    }

    unsafe {
        let mut data = vec![0; 4096];
        let n = data.as_mut_ptr().as_send_mut();
        let x = n.add(5).as_send_mut();
        assert!(x > n);
        assert!(n < x);
        assert_ne!(n, x);
        assert_eq!(n, n);
        assert_eq!(x, x);
    }

    unsafe {
        let mut data = vec![0; 4096];
        let n = data.as_mut_ptr().as_send_const();
        let x = n.add(5).as_send_const();
        assert!(x > n);
        assert!(n < x);
        assert_ne!(n, x);
        assert_eq!(n, n);
        assert_eq!(x, x);
    }

    unsafe {
        let mut data = vec![0; 4096];
        let n = data.as_mut_ptr().as_sync_const();
        let x = n.add(5).as_sync_const();
        assert!(x > n);
        assert!(n < x);
        assert_ne!(n, x);
        assert_eq!(n, n);
        assert_eq!(x, x);
    }
}

#[test]
fn test() {
    unsafe {
        assert_eq!(size_of::<SyncConstPtr<c_void>>(), size_of::<*mut c_void>());
        assert_eq!(
            align_of::<SyncConstPtr<c_void>>(),
            align_of::<*mut c_void>()
        );

        let mut value = 45;
        let n: *mut u64 = &mut value;
        let x = n.as_sync_const();
        assert_eq!(x, x);
        let y = n.as_send_mut();
        let z = y.as_sync_const();
        let u = z.as_send_mut();
        assert_eq!(x, z);
        assert_eq!(u, u);

        assert_eq!(z.read(), 45);
        assert_eq!(z.inner().read(), 45);
        std::thread::spawn(move || assert!(!u.inner().is_null()))
            .join()
            .unwrap();
    }
}

struct RustControlStructureThatIsNowSend {
    some_handle: SendConstPtr<c_void>,
    some_rust_data: u64,
}

#[test]
fn example() {
    //Some handle or pointer obtained goes here.
    let handle: *mut c_void = null_mut();
    let data: u64 = 123u64;

    let rcs = RustControlStructureThatIsNowSend {
        some_handle: unsafe { handle.as_send_const() },
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
        let _send_const_new: SendMutPtr<c_void> = SendMutPtr::new(null_mut());
        //This is unsafe.
    }

    std::thread::spawn(move || {
        assert!(rcs.some_handle.is_null()); //Use boxed
        let _unwrapped: *const c_void = rcs.some_handle.inner(); //unwrap if you want
        let _unwrapped2: *const c_void = rcs.some_handle.into(); //Into<*const T> is also implemented. (*mut T too when applicable)
        let casted: SendConstPtr<usize> = rcs.some_handle.cast::<usize>(); //Cast if you want.
        unsafe {
            if !casted.is_null() {
                //In this example this is obviously always null...
                casted.read_volatile(); //Read if you want
            }
        }

        assert_eq!(rcs.some_rust_data, 123u64)
    })
    .join()
    .unwrap();
}

#[test]
fn test_fmt() {
    unsafe {
        let mut memory = vec![0u8; 12345];
        let handle: *mut c_void = memory.as_mut_ptr().add(12345).cast();
        assert_eq!(
            format!("{:p}", handle.as_send_const()).as_str(),
            format!("{:p}", handle).as_str()
        );
        assert_eq!(
            format!("{:p}", handle.as_send_mut()).as_str(),
            format!("{:p}", handle).as_str()
        );
        assert_eq!(
            format!("{:p}", handle.as_sync_mut()).as_str(),
            format!("{:p}", handle).as_str()
        );
        assert_eq!(
            format!("{:p}", handle.as_sync_const()).as_str(),
            format!("{:p}", handle).as_str()
        );
        assert_ne!(
            format!("{:p}", handle.as_sync_const()).as_str(),
            format!("{:p}", memory.as_ptr()).as_str()
        );
    }
}

#[cfg(target_has_atomic = "32")]
#[test]
fn test_mt() {
    use core::sync::atomic::AtomicU32;
    use core::sync::atomic::Ordering::SeqCst;

    unsafe {
        let n = AtomicU32::new(123);
        let ptr = n.as_ptr().as_sync_mut();
        let jh = std::thread::spawn(move || {
            assert_eq!(123, ptr.read_volatile());
            ptr.write_volatile(456);
        });
        jh.join().unwrap();
        assert_eq!(n.load(SeqCst), 456);
    }
}
