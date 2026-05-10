//! CLI definition for `kalidoku-worker`.
//!
//! Three mutually exclusive modes : `--cron`, `--once`, `--queue`.
//! `--once` accepts an optional `--domain` filter to regenerate a single domain.

use clap::{ArgGroup, Parser};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "kalidoku-worker",
    about = "kalidoku grid generator (cron + on-demand)",
    version
)]
#[command(group(ArgGroup::new("run-mode").required(true).args(["cron", "once", "queue"])))]
pub struct Cli {
    /// Long-running scheduler. Triggers `--once` every day at 00:01 UTC.
    #[arg(long)]
    pub cron: bool,

    /// Run a single generation pass for every active domain and exit.
    #[arg(long)]
    pub once: bool,

    /// Consume the Redis `solo:queue` and generate solo grids on demand.
    #[arg(long)]
    pub queue: bool,

    /// Restrict `--once` to a single domain id (e.g. `paris-metro`).
    #[arg(long, requires = "once")]
    pub domain: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Mode {
    Cron,
    Once { domain: Option<String> },
    Queue,
}

impl Cli {
    pub fn mode(&self) -> Mode {
        if self.cron {
            Mode::Cron
        } else if self.queue {
            Mode::Queue
        } else {
            Mode::Once {
                domain: self.domain.clone(),
            }
        }
    }
}
