use super::*;

/// Show the current value of the counter.
#[derive(Parser)]
pub struct ShowCounter {}

impl Leaf for ShowCounter {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;
        let counter = db.get_counter()?;
        println!("{}", counter.unwrap_or(0));
        Ok(())
    }
}

/// Count until the counter reaches a minimum value.
#[derive(Parser)]
pub struct CountCounter {
    /// The minimum value which counting aims at.
    #[clap(long)]
    goal: i64,
}

impl CountCounter {
    fn inc(db: &Db, goal: i64) -> Result<(), DbError> {
        let mut prev: i64 = -1;
        loop {
            db.begin()?;
            println!("BEGIN");

            let current = db.get_counter()?;
            println!("  current as read={current:?}");
            let current = current.unwrap_or(0);
            println!("  current: {current}; prev={prev}");
            if current < prev {
                panic!("current < prev");
            }
            if current >= goal {
                println!("GOAL");
                db.rollback()?;
                println!("ROLLBACK");
                break;
            }

            let new = current + 1;
            if (new == 1 && db.create_counter(new).is_err()) || db.update_counter(new).is_err() {
                db.rollback()?;
                println!("ROLLBACK");
            } else {
                println!("  increment to {new}");
                db.commit()?;
                println!("COMMIT");
                prev = new;
            }
        }

        Ok(())
    }
}

impl Leaf for CountCounter {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;
        Self::inc(&db, self.goal)?;

        Ok(())
    }
}
