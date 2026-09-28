A versatile and easy to use defer statement for Rust. Similar to Go's or Zig's defer.

Loosely based on and inspired by:
https://github.com/rodrigocfd/defer-lite

This crate is compatible uses `no_std`.
The default features use `alloc`.
To disable alloc set default-features to false in cargo.toml.

This crates provides 6 macros for different use cases of deferment:
1. `defer!` simple deferment. Will execute when current scope ends.
    - If this is all you need then use the `defer-lite` crate!

2. `defer_move!` same as `defer!` but moves local variables into the closure.

3. `defer_guard!` Returns a guard that causes execution when its scope ends.
    - Execution can be canceled or preempted.

4. `defer_move_guard!` Same as `defer_guard!` but moves local variables into the closure.

5. `defer_arc!` Returns a reference counted guard than can be shared with other threads.
    - Execution can be canceled or preempted.
    - Closure must be `Send`
    - Target must support Arc & AtomicBool.
    - Target must support alloc
    - can be disabled with `default-features=false` in Cargo.toml

6. `defer_move_arc!` Same as `defer_arc!` but moves local variables into the closure.
    - All used local variables must be `Send`.

# Usage

Add the dependency in your `Cargo.toml`:

```toml
[dependencies]
defer-heavy = "0.1.0"
```

## Examples

### Simple Defer
If this is all you need use the `defer-lite` crate instead!
```rust
use defer_heavy::defer;

fn main() {
    defer! { println!("Second"); }
    println!("First");
}
```

### Canceled Defer

```rust
use defer_heavy::defer_guard;

fn main() {
    let defer1 = defer_guard! { unreachable!("Wont be executed"); };
    let defer2 = defer_guard! { println!("Second"); };
    let defer3 = defer_guard! { println!("Fourth"); };

    println!("First");
    defer2.destroy(); //Same as drop(defer2);
    println!("Third");
    defer1.cancel();
}
```


### Reference Counted Defer
```rust
use std::thread;
use std::time::Duration;
use defer_heavy::defer_move_arc;

pub fn main() {
    let deferred = defer_arc! { println!("Executed in {:?}", thread::current().id());};
    println!("Main thread {:?}", thread::current().id());
    {
        let deferred = deferred.clone();
        thread::spawn(move ||{
            println!("Spawned thread {:?}", thread::current().id());
            let _deferred = deferred.own();
            thread::sleep(Duration::from_millis(2000)); //SIMULATE work
       });
   }
   thread::sleep(Duration::from_millis(2000)); //SIMULATE WORK
 }
```
Prints:
```text
Main thread Thread(1)
Spawned thread Thread(2)
"Executed in Thread(1)" or "Executed in Thread(2)"
```

# Order of execution
Rust guarantees that the order in which the closures are dropped
(and therefore executed) are in reverse order of creation.
This means the last `defer!` in the scope executes first.

