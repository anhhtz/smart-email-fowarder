use clap::Parser;
use smart_email_forwarder::db::Database;
use smart_email_forwarder::services::poller::PollerManager;
use smart_email_forwarder::web;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(
    name = "smart-email-forwarder",
    author = "Smart Email Forwarder Team",
    version = "0.1.0",
    about = "Minimal, reliable MS Exchange to Gmail forwarder with embedded Web UI"
)]
struct Args {
    /// Port for the Web UI and API
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Bind IP address
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Path to SQLite database file
    #[arg(long, default_value = "data/forwarder.db")]
    db: PathBuf,

    /// Run as a Windows Service (invoked by Service Control Manager)
    #[arg(long, default_value_t = false)]
    service: bool,

    /// Register as a Windows Service (Windows only)
    #[arg(long, default_value_t = false)]
    install_service: bool,

    /// Unregister Windows Service (Windows only)
    #[arg(long, default_value_t = false)]
    uninstall_service: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let args = Args::parse();

    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "smart_email_forwarder=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Handle service installation/uninstallation helpers if on Windows
    #[cfg(windows)]
    {
        if args.install_service {
            return windows_install_service();
        }
        if args.uninstall_service {
            return windows_uninstall_service();
        }
        if args.service {
            return windows_run_service(args.port, args.host, args.db);
        }
    }

    // Normal interactive console mode (runs on Windows Server 2019, Linux Mint, macOS)
    run_app(args.port, &args.host, args.db).await
}

async fn run_app(port: u16, host: &str, db_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Smart Email Forwarder...");
    info!("Database path: {:?}", db_path);

    // Initialize Database
    let db = Database::new(&db_path)?;
    let _ = db.log_event("INFO", "Application starting up.");

    // Initialize Background Poller
    let (poller, sync_rx) = PollerManager::new(db.clone());
    let poller_arc = Arc::new(poller);
    poller_arc.start_background_loop(sync_rx);

    // Build Web Router
    let app = web::build_router(db.clone(), Arc::clone(&poller_arc));

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Web UI accessible at http://{}:{}", host, port);
    println!("\n========================================================");
    println!("  Smart Email Forwarder is running!");
    println!("  Web UI: http://localhost:{}", port);
    if host == "0.0.0.0" {
        println!("  Remote: http://<server-ip>:{}", port);
    }
    println!("  Press Ctrl+C to stop.");
    println!("========================================================\n");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Smart Email Forwarder stopped cleanly.");
    let _ = db.log_event("INFO", "Application shutdown cleanly.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("Received Ctrl+C, shutting down..."); },
        _ = terminate => { info!("Received termination signal, shutting down..."); },
    }
}

// Windows Service Integration
#[cfg(windows)]
fn windows_install_service() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    let exe = std::env::current_exe()?;
    let bin_path = format!("\"{}\" --service", exe.display());

    println!("Registering SmartEmailForwarder Windows Service...");
    let output = Command::new("sc.exe")
        .args(&[
            "create",
            "SmartEmailForwarder",
            &format!("binPath= {}", bin_path),
            "start=",
            "auto",
            "DisplayName=",
            "Smart Email Forwarder",
        ])
        .output()?;

    if output.status.success() {
        println!("Service 'SmartEmailForwarder' registered successfully!");
        println!("Configuring auto-recovery...");
        let _ = Command::new("sc.exe")
            .args(&[
                "failure",
                "SmartEmailForwarder",
                "reset=",
                "86400",
                "actions=",
                "restart/5000/restart/5000/restart/5000",
            ])
            .output();

        println!("Starting service...");
        let _ = Command::new("sc.exe")
            .args(&["start", "SmartEmailForwarder"])
            .output();
        println!("Service started! Access Web UI at http://localhost:8080");
    } else {
        eprintln!(
            "Failed to register service: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(windows)]
fn windows_uninstall_service() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    println!("Stopping and removing SmartEmailForwarder service...");
    let _ = Command::new("sc.exe")
        .args(&["stop", "SmartEmailForwarder"])
        .output();
    let output = Command::new("sc.exe")
        .args(&["delete", "SmartEmailForwarder"])
        .output()?;

    if output.status.success() {
        println!("Service 'SmartEmailForwarder' uninstalled successfully.");
    } else {
        eprintln!(
            "Failed to delete service: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(windows)]
fn windows_run_service(
    port: u16,
    host: String,
    db: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::OsString;
    use std::time::Duration;
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    define_windows_service!(ffi_service_main, my_service_main);

    fn my_service_main(_arguments: Vec<OsString>) {
        if let Err(e) = run_service() {
            error!("Windows Service error: {:?}", e);
        }
    }

    fn run_service() -> Result<(), Box<dyn std::error::Error>> {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    let _ = shutdown_tx.blocking_send(());
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        let status_handle =
            service_control_handler::register("SmartEmailForwarder", event_handler)?;

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            exit_code: ServiceExitCode::NO_ERROR,
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;

        // Start runtime and app
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let app_task = tokio::spawn(async move {
                let _ = run_app(8080, "0.0.0.0", PathBuf::from("data/forwarder.db")).await;
            });

            shutdown_rx.recv().await;
            app_task.abort();
        });

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::NO_ERROR,
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;

        Ok(())
    }

    service_dispatcher::start("SmartEmailForwarder", ffi_service_main)?;
    Ok(())
}
