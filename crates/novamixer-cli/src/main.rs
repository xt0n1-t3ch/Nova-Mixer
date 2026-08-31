use clap::{Parser, Subcommand};
use novamixer_application::NovaMixerApplication;
use serde::Serialize;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long, value_names = ["APP_KEY", "SCALAR"], num_args = 2)]
        set_volume: Option<Vec<String>>,
        #[arg(long)]
        watch: bool,
    },
}
#[derive(Serialize)]
struct Doctor {
    data_root: String,
    migration_ran: bool,
    default_endpoint_name: String,
    live_session_count: usize,
    sessions: Vec<Session>,
    loaded_group_count: usize,
}
#[derive(Serialize)]
struct Session {
    app_key: String,
    volume: f32,
    state: String,
    controllable: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor {
            json,
            set_volume,
            watch,
        } => {
            let started = std::sync::Arc::new(std::sync::Mutex::new(None::<std::time::Instant>));
            let event_clock = started.clone();
            let app = NovaMixerApplication::start(move |event| {
                if watch {
                    let elapsed = event_clock
                        .lock()
                        .ok()
                        .and_then(|value| value.as_ref().map(std::time::Instant::elapsed))
                        .map_or(0, |value| value.as_millis());
                    match event {
                        audio_sessions::AudioEvent::SessionAdded(session) => eprintln!(
                            "event +{}ms session-added {} volume={:.3}",
                            elapsed, session.app_key, session.volume
                        ),
                        audio_sessions::AudioEvent::SessionUpdated(session) => eprintln!(
                            "event +{}ms session-updated {} volume={:.3}",
                            elapsed, session.app_key, session.volume
                        ),
                        audio_sessions::AudioEvent::SessionRemoved { live_id } => {
                            eprintln!("event +{}ms session-removed {}", elapsed, live_id)
                        }
                        _ => {}
                    }
                }
            })?;
            let mut snapshot = app.snapshot().await?;
            if let Some(args) = set_volume {
                let scalar: f32 = args[1].parse()?;
                for session in snapshot
                    .sessions
                    .iter()
                    .filter(|session| session.app_key.eq_ignore_ascii_case(&args[0]))
                {
                    app.audio
                        .set_session_volume(session.live_id.clone(), scalar)
                        .await?;
                }
                snapshot = app.snapshot().await?;
            }
            let report = Doctor {
                data_root: app.data_root().display().to_string(),
                migration_ran: app.migration_ran(),
                default_endpoint_name: snapshot.master.endpoint_name,
                live_session_count: snapshot.sessions.len(),
                sessions: snapshot
                    .sessions
                    .into_iter()
                    .map(|value| Session {
                        app_key: value.app_key,
                        volume: value.volume,
                        state: format!("{:?}", value.state).to_lowercase(),
                        controllable: value.controllable,
                    })
                    .collect(),
                loaded_group_count: app.settings().groups.len(),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Data root: {}\nMigration ran: {}\nDefault endpoint: {}\nLive sessions: {}",
                    report.data_root,
                    report.migration_ran,
                    report.default_endpoint_name,
                    report.live_session_count
                );
                for session in report.sessions {
                    println!(
                        "  {} volume={:.3} state={} controllable={}",
                        session.app_key, session.volume, session.state, session.controllable
                    );
                }
                println!("Loaded groups: {}", report.loaded_group_count);
            }
            if watch {
                if let Ok(mut value) = started.lock() {
                    *value = Some(std::time::Instant::now());
                }
                eprintln!("Watching Core Audio callbacks. Press Ctrl+C to stop.");
                tokio::signal::ctrl_c().await?;
            }
        }
    }
    Ok(())
}
