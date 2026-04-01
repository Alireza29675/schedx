mod backend;
mod cli;
mod commands;
mod engine;
mod model;
mod output;
mod schedule;
mod store;
mod upgrade;
mod util;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Commands};

#[allow(clippy::too_many_lines)]
fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Add {
            schedule,
            run,
            prompt,
            webhook,
            name,
            tag,
            timeout,
            workdir,
            agent,
            stdin,
            shell,
            method,
            header,
            body,
            on_failure,
            on_failure_shell,
        } => commands::add::execute(
            schedule,
            run.as_deref(),
            prompt.as_deref(),
            webhook.as_deref(),
            name.as_deref(),
            tag,
            *timeout,
            workdir.as_deref(),
            agent.as_deref(),
            *stdin,
            *shell,
            method.as_deref(),
            header,
            body.as_deref(),
            on_failure.as_deref(),
            *on_failure_shell,
            cli.json,
        ),

        Commands::List { status, tag, all } => {
            commands::list::execute(status.as_deref(), tag.as_deref(), *all, cli.json)
        }

        Commands::Get { id } => commands::get::execute(id, cli.json),

        Commands::Edit {
            id,
            name,
            prompt,
            prompt_stdin,
            run,
            agent,
            timeout,
            schedule,
        } => commands::edit::execute(
            id,
            name.as_deref(),
            prompt.as_deref(),
            *prompt_stdin,
            run.as_deref(),
            agent.as_deref(),
            *timeout,
            schedule.as_deref(),
            cli.json,
        ),

        Commands::Run { id } => commands::run::execute(id),

        Commands::Rm { id, force } => commands::rm::execute(id, *force),

        Commands::Pause { id } => commands::pause::execute(id),

        Commands::Resume { id } => commands::resume::execute(id),

        Commands::Unarchive { id } => commands::unarchive::execute(id),

        Commands::Skip { id, times } => commands::skip::execute(id, *times, cli.json),

        Commands::Logs { id, run, lines } => commands::logs::execute(id, run.as_deref(), *lines),

        Commands::History { id, limit } => {
            commands::history::execute(id.as_deref(), *limit, cli.json)
        }

        Commands::Agent { command } => commands::agent::execute(command, cli.json),

        Commands::Skill { command } => commands::skill::execute(command, cli.json),

        Commands::Config { key, value } => {
            commands::config::execute(key.as_deref(), value.as_deref(), cli.json)
        }

        Commands::Repair => commands::repair::execute(cli.json),

        Commands::Daemon { interval } => commands::daemon::execute(*interval),

        Commands::Upgrade { force } => commands::upgrade::execute(*force, cli.json),

        Commands::Dispatch => commands::dispatch::execute(),

        Commands::Exec {
            job_id,
            scheduled_for,
            trigger,
            run_id,
        } => match commands::exec::execute(job_id, scheduled_for, trigger, run_id.as_deref()) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::from(1),
            Err(e) => Err(e),
        },

        Commands::ExecFallback {
            job_id,
            failed_run_id,
            failed_status,
            failed_exit_code,
            failed_log_path,
            failed_scheduled_for,
        } => {
            let _ = crate::store::paths::ensure_dirs();
            match crate::engine::executor::exec_fallback(
                job_id,
                failed_run_id,
                failed_status,
                failed_exit_code,
                failed_log_path,
                failed_scheduled_for,
            ) {
                Ok(_) => return ExitCode::SUCCESS,
                Err(e) => Err(e),
            }
        }
    };

    // Post-command update hint (skip for internal commands, upgrade, and --json mode)
    let is_internal = matches!(
        cli.command,
        Commands::Dispatch
            | Commands::Exec { .. }
            | Commands::ExecFallback { .. }
            | Commands::Upgrade { .. }
    );
    if !is_internal {
        if let Some(hint) = upgrade::check::maybe_hint(cli.json) {
            eprintln!("{hint}");
        }
    }

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let exit_code = classify_error(&e);
            eprintln!("{e:#}");
            ExitCode::from(exit_code)
        }
    }
}

/// Map error messages to exit codes per spec.
fn classify_error(e: &anyhow::Error) -> u8 {
    let msg = format!("{e:#}");

    if msg.contains("not found") {
        3
    } else if msg.contains("Could not parse schedule")
        || msg.contains("Invalid cron")
        || msg.contains("Empty schedule")
        || msg.contains("in the past")
    {
        4
    } else if msg.contains("blocked by default")
        || msg.contains("HTTP webhooks")
        || msg.contains("Only http:// and https://")
    {
        5
    } else if msg.contains("No supported scheduling backend")
        || msg.contains("Unknown backend")
        || msg.contains("lingering is disabled")
    {
        6
    } else if msg.contains("Exactly one action")
        || msg.contains("can only be used with")
        || msg.contains("Invalid status")
        || msg.contains("Unknown config key")
        || msg.contains("exceeds maximum")
    {
        2
    } else {
        1
    }
}
