use std::error::Error;
use std::future::Future;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::put;
use axum::{routing::get, Router};
use dotenv::dotenv;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;
use tokio::{select, signal};
use ws2818_rgb_led_spi_driver::adapter_spi::WS28xxSpiAdapter;

use crate::error::SolarMonitorError;
use crate::rgbdigit::SevenSegmentDisplayString;
use crate::tesla_powerwall::PowerwallApi;
use solar_status::SolarStatusDisplay;

#[cfg(feature = "i2c_display")]
mod i2c_display;

mod solar_status;

#[cfg(not(feature = "i2c_display"))]
mod console_display;

mod error;
mod rgbdigit;
mod rgbdigit_display;
mod tesla_powerwall;

async fn root() -> &'static str {
    "Hello, this is the webserver controller for the solar monitor device. Use PUT /start or PUT /stop to control the state."
}

async fn start_display(State(app_state): State<AppState>) -> impl IntoResponse {
    // @todo refactor these double handling to a common try block once stabilised
    if let Err(e) = app_state.command_sender.send(Command::START).await {
        eprintln!("Failed to send start command {:?}", e.0);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to send start command {:?}", e.0),
        );
    }

    if let Err(e) = app_state.command_sender.send(Command::TICK).await {
        eprintln!("Failed to send tick command {:?}", e.0);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to send tick command {:?}", e.0),
        );
    }

    println!("Starting solar monitor");

    (StatusCode::OK, "Starting solar monitor...".to_string())
}

async fn stop_display(State(app_state): State<AppState>) -> impl IntoResponse {
    if let Err(e) = app_state.command_sender.send(Command::STOP).await {
        eprintln!("Failed to send command {:?}", e.0);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to send command {:?}", e.0),
        );
    }

    println!("Stopping solar monitor");

    (StatusCode::OK, "Stopping solar monitor...".to_string())
}

#[derive(Debug)]
enum Command {
    START,
    STOP,
    TICK,
}

#[derive(Clone)]
struct AppState {
    command_sender: Sender<Command>,
}

async fn process_event_loop<D: SolarStatusDisplay>(
    mut rx: Receiver<Command>,
    mut display: &mut D,
    mut powerwall: PowerwallApi,
) -> Result<(), Box<dyn Error>> {
    let mut output = false;
    display.clear()?;

    while let Some(message) = rx.recv().await {
        match message {
            Command::START => {
                output = true;
                println!("Starting solar monitor");
            }
            Command::TICK => {
                if output {
                    match powerwall.get_stats().await {
                        Ok(status) => {
                            display
                                .show_status(status)
                                .inspect_err(|e| eprintln!("Failed to show status: {:?}", e))
                                .ok();
                        }
                        Err(e) => {
                            println!("Failed to retrieve powerwall status {:?}", &e);
                            display.show_error(&SolarMonitorError::API(e)).ok();
                        }
                    }
                } else {
                    println!("Asleep; ignoring tick");
                }
            }
            Command::STOP => {
                output = false;
                display
                    .shutdown()
                    .inspect_err(|e| eprintln!("Failed to shutdown display: {:?}", e))
                    .ok();
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let (tx, rx) = mpsc::channel(32);

    let webserver_tx = tx.clone();
    let shutdown_tx = tx.clone();

    let ticker = tokio::spawn(async move {
        loop {
            tx.send(Command::TICK).await.expect("Failed to send tick");
            sleep(Duration::from_millis(1000)).await;
        }
    });

    let ctrl_c = async move {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");

        println!("received ctrl+c");

        ticker.abort();

        shutdown_tx.send(Command::STOP).await.unwrap();
    };

    let local = tokio::task::LocalSet::new();
    let local_handle = local.run_until(async move {
        println!("Localset started");

        #[cfg(not(feature = "i2c_display"))]
        let mut display = console_display::ConsoleDisplay {};

        #[cfg(feature = "i2c_display")]
        let adapter = WS28xxSpiAdapter::new("/dev/spidev0.0").expect("Could not open ws28x SPI device");
        #[cfg(feature = "i2c_display")]
        let seven_segment_display = SevenSegmentDisplayString::new(adapter, 10);
        #[cfg(feature = "i2c_display")]
        let mut display = rgbdigit_display::RgbDigitDisplay {
            display: &seven_segment_display,
            solar_generation_status: &mut seven_segment_display.derive_numeric_display(&[2, 3]),
            house_consumption_status: &mut seven_segment_display.derive_numeric_display(&[4, 5]),
            battery_status: &mut seven_segment_display.derive_numeric_display(&[6, 7]),
            grid_status: &mut seven_segment_display.derive_numeric_display(&[0, 1]),
            battery_level: &mut seven_segment_display.derive_numeric_display(&[8, 9]),
        };

        let powerwall = PowerwallApi::new().expect("failed to create powerwall API");

        println!(
            "Connecting to powerwall on {ip}...",
            ip = powerwall.ip_address
        );

        select! {
            _ = display.start_await() => {}
            powerwall_connection_result = powerwall.wait_for_connection() => {
                display.clear().inspect_err(|e| {
                    eprintln!("Failed to retrieve clear screen status {:?}", e);
                }).ok();

                if let Err(err) = powerwall_connection_result {
                    eprintln!("Failed to connect to powerwall {:?}", err);
                    display.show_error(&SolarMonitorError::API(err)).inspect_err(|e| {
                        eprintln!("Failed to show error state {:?}", e);
                    }).ok();
                }
            }
        }

        process_event_loop(rx, &mut display, powerwall).await.unwrap();
    });

    let (_, webserver_result) = tokio::join!(local_handle, webserver(webserver_tx, ctrl_c));

    webserver_result.expect("Webserver should run continuously")
}

async fn webserver<S>(
    webserver_tx: Sender<Command>,
    shutdown_signal: S,
) -> Result<(), Box<dyn Error>>
where
    S: Future<Output = ()> + Send + 'static,
{
    println!("Starting webserver");

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/start", put(start_display))
        .route("/stop", put(stop_display))
        .with_state(AppState {
            command_sender: webserver_tx,
        });

    // run our app with hyper, listening globally on port 3000
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}
