//! `tunelock`: command-line client for the Tunelock service.

mod duration;

use std::io::{BufRead, Write};
use std::time::Duration;

use anyhow::{Context, bail};
use chrono::{NaiveTime, Weekday};
use clap::{Parser, Subcommand};
use tunelock_client::Client;
use tunelock_core::{Allowlist, LockMode, PlaylistId, Preset, Schedule, format_duration};
use tunelock_protocol::{CONTROL_SOCKET, Request, Response, SessionInfo};

use crate::duration::parse_duration;

#[derive(Parser)]
#[command(name = "tunelock", version, about = "Control Tunelock focus sessions")]
struct Cli {
    /// Name of the service's control socket.
    #[arg(long, default_value = CONTROL_SOCKET, hide = true)]
    socket: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the current session.
    Status,
    /// Start a session, e.g. `tunelock start 2h30m --locked`.
    Start {
        /// How long, e.g. 45m, 2h, 1d12h.
        #[arg(value_parser = parse_duration)]
        duration: Duration,
        /// Cannot be cancelled until it ends.
        #[arg(long)]
        locked: bool,
        /// Playlist link or ID (repeatable). Defaults to the saved allowlist.
        #[arg(long = "playlist", value_name = "LINK")]
        playlists: Vec<String>,
        /// Skip the confirmation for locked sessions.
        #[arg(long, short)]
        yes: bool,
    },
    /// End a flexible session.
    Cancel,
    /// Add time to the running session.
    Extend {
        #[arg(value_parser = parse_duration)]
        duration: Duration,
    },
    /// Turn the running flexible session into a locked one.
    Harden {
        #[arg(long, short)]
        yes: bool,
    },
    /// Manage the playlists allowed during sessions.
    #[command(subcommand)]
    Allowlist(AllowlistCommand),
    /// Manage saved presets.
    #[command(subcommand)]
    Preset(PresetCommand),
    /// Manage weekly schedules.
    #[command(subcommand)]
    Schedule(ScheduleCommand),
}

#[derive(Subcommand)]
enum AllowlistCommand {
    Show,
    /// Replace the allowlist with these playlist links or IDs.
    Set {
        #[arg(required = true, value_name = "LINK")]
        playlists: Vec<String>,
    },
}

#[derive(Subcommand)]
enum PresetCommand {
    List,
    Save {
        name: String,
        #[arg(value_parser = parse_duration)]
        duration: Duration,
        #[arg(long)]
        locked: bool,
    },
    Delete {
        name: String,
    },
    /// Start a session from a preset with the saved allowlist.
    Start {
        name: String,
        #[arg(long, short)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum ScheduleCommand {
    List,
    /// e.g. `tunelock schedule add Work --days mon,tue,wed,thu,fri --from 09:00 --to 18:00 --locked`
    Add {
        name: String,
        #[arg(long, value_delimiter = ',', required = true)]
        days: Vec<Weekday>,
        #[arg(long, value_parser = parse_time)]
        from: NaiveTime,
        #[arg(long, value_parser = parse_time)]
        to: NaiveTime,
        #[arg(long)]
        locked: bool,
    },
    Delete {
        name: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut client = Client::connect(&cli.socket)?;
    run(&mut client, cli.command)
}

fn run(client: &mut Client, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Status => match client.status()?.session {
            Some(session) => print_session(&session),
            None => println!("No active session."),
        },
        Command::Start {
            duration,
            locked,
            playlists,
            yes,
        } => {
            let allowlist = if playlists.is_empty() {
                saved_allowlist(client)?
            } else {
                parse_allowlist(&playlists)?
            };
            let mode = mode(locked);
            if mode == LockMode::Locked && !yes {
                confirm_locked(duration)?;
            }
            client.done(&Request::StartSession {
                mode,
                duration,
                allowlist,
            })?;
            print_status(client)?;
        }
        Command::Cancel => {
            client.done(&Request::Cancel)?;
            println!("Session ended.");
        }
        Command::Extend { duration } => {
            client.done(&Request::Extend { by: duration })?;
            print_status(client)?;
        }
        Command::Harden { yes } => {
            let remaining = client
                .status()?
                .session
                .context("no session is active")?
                .remaining;
            if !yes {
                confirm_locked(remaining)?;
            }
            client.done(&Request::Harden)?;
            print_status(client)?;
        }
        Command::Allowlist(AllowlistCommand::Show) => match client.allowlist()? {
            Some(list) => list.iter().for_each(|id| println!("{}", id.uri())),
            None => println!("No playlists allowed yet. Use `tunelock allowlist set <LINK>...`."),
        },
        Command::Allowlist(AllowlistCommand::Set { playlists }) => {
            let allowlist = parse_allowlist(&playlists)?;
            client.done(&Request::SetAllowlist { allowlist })?;
            println!("Allowlist saved.");
        }
        Command::Preset(command) => run_preset(client, command)?,
        Command::Schedule(command) => run_schedule(client, command)?,
    }
    Ok(())
}

fn run_preset(client: &mut Client, command: PresetCommand) -> anyhow::Result<()> {
    match command {
        PresetCommand::List => {
            let Response::Presets { presets } = client.send(&Request::ListPresets)? else {
                bail!("unexpected response");
            };
            if presets.is_empty() {
                println!("No presets.");
            }
            for preset in presets {
                println!(
                    "{}: {} {}",
                    preset.name(),
                    format_duration(&preset.duration()),
                    mode_label(preset.mode())
                );
            }
        }
        PresetCommand::Save {
            name,
            duration,
            locked,
        } => {
            let preset = Preset::new(&name, mode(locked), duration)?;
            client.done(&Request::SavePreset { preset })?;
            println!("Preset saved.");
        }
        PresetCommand::Delete { name } => {
            client.done(&Request::DeletePreset { name })?;
            println!("Preset deleted.");
        }
        PresetCommand::Start { name, yes } => {
            let Response::Presets { presets } = client.send(&Request::ListPresets)? else {
                bail!("unexpected response");
            };
            let preset = presets
                .iter()
                .find(|p| p.name() == name)
                .with_context(|| format!("no preset named {name:?}"))?;
            if preset.mode() == LockMode::Locked && !yes {
                confirm_locked(preset.duration())?;
            }
            client.done(&Request::StartPreset { name })?;
            print_status(client)?;
        }
    }
    Ok(())
}

fn run_schedule(client: &mut Client, command: ScheduleCommand) -> anyhow::Result<()> {
    match command {
        ScheduleCommand::List => {
            let Response::Schedules { schedules } = client.send(&Request::ListSchedules)? else {
                bail!("unexpected response");
            };
            if schedules.is_empty() {
                println!("No schedules.");
            }
            for schedule in schedules {
                let days: Vec<String> = schedule.days().iter().map(ToString::to_string).collect();
                println!(
                    "{}: {} {}-{} {}",
                    schedule.name(),
                    days.join(","),
                    schedule.start().format("%H:%M"),
                    schedule.end().format("%H:%M"),
                    mode_label(schedule.mode())
                );
            }
        }
        ScheduleCommand::Add {
            name,
            days,
            from,
            to,
            locked,
        } => {
            let schedule = Schedule::new(&name, days, from, to, mode(locked))?;
            client.done(&Request::SaveSchedule { schedule })?;
            println!("Schedule saved.");
        }
        ScheduleCommand::Delete { name } => {
            client.done(&Request::DeleteSchedule { name })?;
            println!("Schedule deleted.");
        }
    }
    Ok(())
}

fn mode(locked: bool) -> LockMode {
    if locked {
        LockMode::Locked
    } else {
        LockMode::Flexible
    }
}

fn mode_label(mode: LockMode) -> &'static str {
    match mode {
        LockMode::Flexible => "flexible",
        LockMode::Locked => "locked",
    }
}

fn parse_time(s: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(s, "%H:%M").map_err(|_| format!("expected HH:MM, got {s:?}"))
}

fn parse_allowlist(links: &[String]) -> anyhow::Result<Allowlist> {
    let ids = links
        .iter()
        .map(|link| PlaylistId::parse(link))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Allowlist::new(ids)?)
}

fn saved_allowlist(client: &mut Client) -> anyhow::Result<Allowlist> {
    client.allowlist()?.context(
        "no playlists allowed yet; pass --playlist or run `tunelock allowlist set <LINK>...`",
    )
}

fn confirm_locked(duration: Duration) -> anyhow::Result<()> {
    print!(
        "This session cannot be cancelled for {}. Type \"lock\" to confirm: ",
        format_duration(&duration)
    );
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    if answer.trim() != "lock" {
        bail!("not confirmed; nothing changed");
    }
    Ok(())
}

fn print_status(client: &mut Client) -> anyhow::Result<()> {
    if let Some(session) = client.status()?.session {
        print_session(&session);
    }
    Ok(())
}

fn print_session(session: &SessionInfo) {
    println!(
        "Active ({}): {} remaining of {}",
        mode_label(session.mode),
        format_duration(&session.remaining),
        format_duration(&session.duration)
    );
    for id in session.allowlist.iter() {
        println!("  {}", id.uri());
    }
}
