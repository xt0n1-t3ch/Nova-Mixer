use clap::{Parser, Subcommand};
use novamixer_application::NovaMixerApplication;
use serde::Serialize;
use std::time::Duration;

mod efficiency;

#[derive(Parser)]
struct Cli {
    #[arg(long, hide = true, value_parser = ["on", "off"])]
    efficiency: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long,value_names=["APP_KEY","SCALAR"],num_args=2)]
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
    application_count: usize,
    live_session_count: usize,
    applications: Vec<App>,
    loaded_group_count: usize,
}
#[derive(Serialize)]
struct App {
    app_key: String,
    display_name: String,
    volume: f32,
    running: bool,
    session_count: usize,
    controllable: bool,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    if let Some(mode) = cli.efficiency {
        let proof = efficiency::apply(mode == "on")?;
        println!("GetPriorityClass: 0x{:08X}", proof.priority_class);
        println!(
            "GetProcessInformation(ProcessPowerThrottling).StateMask: 0x{:08X}",
            proof.ecoqos_state_mask
        );
        println!("Low base priority: {}", proof.low_priority);
        println!("EcoQoS execution-speed throttling: {}", proof.ecoqos);
        tokio::time::sleep(Duration::from_secs(10)).await;
        return Ok(());
    }
    match cli.command.unwrap_or(Command::Doctor {
        json: false,
        set_volume: None,
        watch: false,
    }) {
        Command::Doctor {
            json,
            set_volume,
            watch,
        } => {
            let app = NovaMixerApplication::start(move |event| {
                if watch {
                    match event {
                        audio_sessions::AudioEvent::ApplicationAdded(a) => eprintln!(
                            "application-added {} sessions={}",
                            a.app_key,
                            a.sessions.len()
                        ),
                        audio_sessions::AudioEvent::ApplicationUpdated(a) => eprintln!(
                            "application-updated {} sessions={}",
                            a.app_key,
                            a.sessions.len()
                        ),
                        audio_sessions::AudioEvent::ApplicationRemoved { app_key } => {
                            eprintln!("application-removed {app_key}")
                        }
                        _ => {}
                    }
                }
            })?;
            if let Some(args) = set_volume {
                app.set_app_volume(&args[0], args[1].parse()?).await?
            }
            let snapshot = app.snapshot().await?;
            let live = snapshot.applications.iter().map(|a| a.sessions.len()).sum();
            let report = Doctor {
                data_root: app.data_root().display().to_string(),
                migration_ran: app.migration_ran(),
                default_endpoint_name: snapshot.master.endpoint_name,
                application_count: snapshot.applications.len(),
                live_session_count: live,
                applications: snapshot
                    .applications
                    .into_iter()
                    .map(|a| App {
                        app_key: a.app_key,
                        display_name: a.display_name,
                        volume: a.volume,
                        running: a.running,
                        session_count: a.sessions.len(),
                        controllable: a.controllable,
                    })
                    .collect(),
                loaded_group_count: app.settings().groups.len(),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?)
            } else {
                println!("Data root: {}\nMigration ran: {}\nDefault endpoint: {}\nApplications: {}\nLive sessions: {}",report.data_root,report.migration_ran,report.default_endpoint_name,report.application_count,report.live_session_count);
                for a in report.applications {
                    println!(
                        "  {} ({}) volume={:.3} running={} sessions={} controllable={}",
                        a.display_name,
                        a.app_key,
                        a.volume,
                        a.running,
                        a.session_count,
                        a.controllable
                    )
                }
                println!("Loaded groups: {}", report.loaded_group_count)
            }
            if watch {
                eprintln!("Watching Core Audio callbacks. Press Ctrl+C to stop.");
                tokio::signal::ctrl_c().await?
            }
        }
    }
    Ok(())
}
