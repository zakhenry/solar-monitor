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

use solar_status::SolarStatusDisplay;

use crate::rgbdigit::SevenSegmentDisplayString;
use crate::tesla_powerwall::PowerwallApi;

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

async fn display(mut rx: Receiver<Command>) -> Result<(), Box<dyn Error>> {
    #[cfg(not(feature = "i2c_display"))]
    let mut display = console_display::ConsoleDisplay {};

    #[cfg(feature = "i2c_display")]
    let adapter = WS28xxSpiAdapter::new("/dev/spidev0.0")?;
    let seven_segment_display = SevenSegmentDisplayString::new(adapter, 10);
    let mut display = rgbdigit_display::RgbDigitDisplay {
        display: &seven_segment_display,
        solar_generation_status: &mut seven_segment_display.derive_numeric_display(&[4, 5]),
        house_consumption_status: &mut seven_segment_display.derive_numeric_display(&[6, 7]),
        battery_status: &mut seven_segment_display.derive_numeric_display(&[0, 1]),
        grid_status: &mut seven_segment_display.derive_numeric_display(&[2, 3]),
        battery_level: &mut seven_segment_display.derive_numeric_display(&[8, 9]),
    };

    let mut powerwall = PowerwallApi::new()?;

    select! {
        _ = display.start_await() => {}
        _ = powerwall.wait_for_connection() => {
            // ensure the display is cleared before continuing
            // (otherwise the cancellation might have left a startup state on the display)
            display.clear()?
        }
    }

    let mut output = false;
    display.clear()?;

    while let Some(message) = rx.recv().await {
        let result = match message {
            Command::START => {
                output = true;
                Ok(())
            }
            Command::TICK => {
                if output {
                    let status = powerwall.get_stats().await?;

                    display.show_status(status)
                } else {
                    println!("Asleep; ignoring tick");
                    Ok(())
                }
            }
            Command::STOP => {
                output = false;
                display.shutdown()
            }
        };

        println!("{:?} result: {:?}", message, result);
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

        tokio::task::spawn_local(display(rx))
            .await
            .unwrap()
            .unwrap();
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
