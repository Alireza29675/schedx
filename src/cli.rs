use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "schedx",
    version,
    about = "Secure and reliable scheduler CLI for commands, prompts, and webhooks"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output as JSON (for supported commands)
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Add a new scheduled job
    Add {
        /// Schedule expression (cron, 'in Xm/h/d', 'every Xm/h/d', or ISO-8601)
        schedule: String,

        /// Command to run
        #[arg(long)]
        run: Option<String>,

        /// Prompt text for an agent
        #[arg(long)]
        prompt: Option<String>,

        /// Webhook URL to call
        #[arg(long)]
        webhook: Option<String>,

        /// Job name
        #[arg(long)]
        name: Option<String>,

        /// Tag (repeatable)
        #[arg(long, action = clap::ArgAction::Append)]
        tag: Vec<String>,

        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,

        /// Working directory (run action only)
        #[arg(long)]
        workdir: Option<String>,

        /// Agent name for prompt action
        #[arg(long)]
        agent: Option<String>,

        /// Read command/prompt from stdin
        #[arg(long)]
        stdin: bool,

        /// Use shell execution (/bin/sh -lc)
        #[arg(long)]
        shell: bool,

        /// HTTP method for webhook (GET, POST, PUT, PATCH, DELETE)
        #[arg(long)]
        method: Option<String>,

        /// HTTP header (repeatable, format: "Key: Value")
        #[arg(long, action = clap::ArgAction::Append)]
        header: Vec<String>,

        /// Request body for webhook
        #[arg(long)]
        body: Option<String>,
    },

    /// List scheduled jobs
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },

    /// Show details for a specific job
    Get {
        /// Job ID or name
        id: String,
    },

    /// Run a job immediately (manual trigger)
    Run {
        /// Job ID or name
        id: String,
    },

    /// Edit an existing job's properties
    Edit {
        /// Job ID or name
        id: String,

        /// New name for the job
        #[arg(long)]
        name: Option<String>,

        /// New prompt text (prompt jobs only)
        #[arg(long)]
        prompt: Option<String>,

        /// Read new prompt from stdin (prompt jobs only)
        #[arg(long)]
        prompt_stdin: bool,

        /// New command (run jobs only)
        #[arg(long)]
        run: Option<String>,

        /// New agent name (prompt jobs only)
        #[arg(long)]
        agent: Option<String>,

        /// New timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,

        /// New schedule expression
        #[arg(long)]
        schedule: Option<String>,
    },

    /// Remove a job
    Rm {
        /// Job ID or name
        id: String,

        /// Force removal without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Pause a job's schedule
    Pause {
        /// Job ID or name
        id: String,
    },

    /// Resume a paused job
    Resume {
        /// Job ID or name
        id: String,
    },

    /// Skip the next N scheduled runs of a recurring job
    Skip {
        /// Job ID or name
        id: String,

        /// Number of runs to skip (default: 1)
        #[arg(long, default_value = "1")]
        times: u32,
    },

    /// View job logs
    Logs {
        /// Job ID or name
        id: String,

        /// Specific run ID
        #[arg(long)]
        run: Option<String>,

        /// Number of lines to show (from end)
        #[arg(long)]
        lines: Option<usize>,
    },

    /// View run history
    History {
        /// Job ID (optional, show all if omitted)
        id: Option<String>,

        /// Maximum number of records
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Manage agent profiles
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },

    /// Read or set configuration
    Config {
        /// Config key
        key: Option<String>,

        /// Config value (set mode)
        value: Option<String>,
    },

    /// Repair backend and state
    Repair,

    /// Run the scheduler daemon in the foreground
    Daemon {
        /// Dispatch interval in seconds (default: 10)
        #[arg(long)]
        interval: Option<u64>,
    },

    /// Internal: dispatch tick (hidden)
    #[command(hide = true)]
    #[command(name = "_dispatch")]
    Dispatch,

    /// Internal: execute a single job (hidden)
    #[command(hide = true)]
    #[command(name = "_exec")]
    Exec {
        /// Job ID
        job_id: String,

        /// Scheduled-for timestamp (RFC-3339)
        #[arg(long)]
        scheduled_for: String,

        /// Trigger type
        #[arg(long, default_value = "scheduled")]
        trigger: String,

        /// Claimed run ID (scheduled runs only)
        #[arg(long)]
        run_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum AgentCommands {
    /// Add a new agent profile
    Add {
        /// Agent name
        name: String,

        /// Path to agent binary
        #[arg(long)]
        bin: String,

        /// Additional arguments (repeatable)
        #[arg(long, action = clap::ArgAction::Append)]
        arg: Vec<String>,

        /// Pass prompt via stdin instead of argument
        #[arg(long)]
        prompt_stdin: bool,
    },

    /// Remove an agent profile
    Rm {
        /// Agent name
        name: String,
    },

    /// List agent profiles
    List,

    /// Set the default agent
    Default {
        /// Agent name
        name: String,
    },
}
