use defer_heavy::{defer, defer_guard, defer_move, defer_move_guard};
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "mt")]
mod mt_test {

    use defer_heavy::{
        defer, defer_arc, defer_guard, defer_move, defer_move_arc, defer_move_guard,
    };
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering::SeqCst;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    pub fn test_mt() {
        let destroyed = Arc::new(AtomicBool::new(false));
        let des = destroyed.clone();
        let deferred = defer_move_arc! {
            println!("Destructor executed in {:?}", thread::current().id());
            //Unless your system is abysmally slow this will be the spawned thread.
            assert_eq!(des.swap(true, SeqCst), false)
        };

        println!("Test thread {:?}", thread::current().id());

        let jh;
        {
            let deferred = deferred.clone();
            let destroyed = destroyed.clone();
            jh = thread::spawn(move || {
                println!("Spawned thread {:?}", thread::current().id());
                let _deferred = deferred.own();
                thread::sleep(Duration::from_millis(2000));
                assert_eq!(destroyed.load(SeqCst), false);
            });
        }

        drop(deferred);
        assert_eq!(destroyed.load(SeqCst), false);
        jh.join().unwrap();
        assert_eq!(destroyed.load(SeqCst), true);
    }

    #[test]
    pub fn test_mt_example() {
        let deferred =
            defer_arc! { println!("Destructor executed in {:?}", thread::current().id());};
        println!("Main thread {:?}", thread::current().id());
        let jh;
        {
            let deferred = deferred.clone();
            jh = thread::spawn(move || {
                println!("Spawned thread {:?}", thread::current().id());
                let _deferred = deferred.own();
                thread::sleep(Duration::from_millis(2000)); //SIMULATE work
            });
        }
        thread::sleep(Duration::from_millis(2000)); //SIMULATE WORK
        drop(deferred);
        jh.join().unwrap();
    }

    #[test]
    pub fn test_macros_compile() {
        defer! {
            println!("HI1");
        }

        defer_move! {
            println!("HI2");
        }

        let _guard = defer_guard! {
            println!("HI3");
        };

        let _guard = defer_move_guard! {
            println!("HI4");
        };

        let _guard = defer_arc! {
            println!("HI5");
        };

        let _guard = defer_move_arc! {
            println!("HI6");
        };
    }
}

#[test]
pub fn test_defer_cancel() {
    let destroyed = Rc::new(RefCell::new(false));
    let des = destroyed.clone();
    let deferred = defer_move_guard! {
        assert_eq!(des.replace(true), false)
    };

    assert_eq!(destroyed.borrow().clone(), false);
    drop(deferred);
    assert_eq!(destroyed.borrow().clone(), true);
}

#[test]
pub fn test_defer_cancel2() {
    let destroyed = Rc::new(RefCell::new(false));
    let des = destroyed.clone();
    let mut deferred = defer_move_guard! {
        assert_eq!(des.replace(true), false)
    };

    assert_eq!(*destroyed.borrow(), false);
    assert_eq!(deferred.cancel_ref(), true);
    drop(deferred);
    assert_eq!(*destroyed.borrow(), false);
}

#[test]
pub fn test_defer() {
    let destroyed = Rc::new(RefCell::new(false));
    {
        let des = destroyed.clone();
        defer_move! {
            assert_eq!(des.replace(true), false)
        }

        assert_eq!(destroyed.borrow().clone(), false);
    }

    assert_eq!(destroyed.borrow().clone(), true);
}

#[test]
pub fn test_defer_muti() {
    let destroyed = Rc::new(RefCell::new(0u8));
    {
        let des = destroyed.clone();
        defer_move! {
            assert_eq!(des.replace(2), 1)
        }

        let des = destroyed.clone();
        defer_move! {
            assert_eq!(des.replace(1), 0)
        }

        assert_eq!(destroyed.borrow().clone(), 0);
    }

    assert_eq!(destroyed.borrow().clone(), 2);
}

#[test]
pub fn test_macros_compile() {
    defer! {
        println!("HI1");
    }

    defer_move! {
        println!("HI2");
    }

    let _guard = defer_guard! {
        println!("HI3");
    };

    let _guard = defer_move_guard! {
        println!("HI4");
    };
}
