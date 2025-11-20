//! Model project roadmaps and step dependencies
//!
//! This crate models a roadmap as steps, which may depend on each
//! other, and a directed acyclic graph (DAG) of said steps, which
//! leads to the end goal. There is no support for due dates,
//! estimates, or other such project management features. These roadmaps only
//! care about what steps need to be take, in what order, to reach the
//! goal.
//!
//! # Example
//! ```
//! # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//! let mut r = roadmap::from_yaml("
//! endgoal:
//!   label: The end goal
//!   depends:
//!   - first
//! first:
//!   label: The first step
//! ").unwrap();
//!
//! let n: Vec<&str> = r.step_names().collect();
//! assert_eq!(n.len(), 2);
//! assert!(n.contains(&"first"));
//! assert!(n.contains(&"endgoal"));
//!
//! r.set_missing_statuses();
//! println!("{}", r.format_as_dot(30).unwrap());
//!
//! # Ok(())
//! # }
//! ```

mod err;
pub use err::RoadmapError;

mod status;
pub use status::Status;

mod step;
pub use step::Step;

mod map;
pub use map::Roadmap;
pub use map::RoadmapResult;

mod parser;
pub use parser::from_yaml;
