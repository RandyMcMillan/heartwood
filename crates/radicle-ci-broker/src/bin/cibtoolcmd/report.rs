use radicle_ci_broker::pages::StatusPage;

use super::*;

/// Produce HTML reports based on database contents.
#[derive(Parser)]
pub struct ReportCmd {
    /// Write HTML files to this directory. The directory must exist:
    /// it is not created automatically.
    #[clap(long)]
    output_dir: PathBuf,
}

impl Leaf for ReportCmd {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let profile = Profile::load().map_err(CibToolError::Profile)?;

        let db = args.open_db()?;

        let mut page = StatusPage::default();
        page.set_output_dir(&self.output_dir);

        let mut run_notification = NotificationChannel::new_run();
        let thread = page.update_in_thread(
            run_notification.rx().map_err(CibToolError::Notification)?,
            profile,
            db,
            true,
        );
        thread.join().unwrap()?;

        Ok(())
    }
}
