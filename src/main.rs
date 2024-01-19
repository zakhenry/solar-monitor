use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{thread, time};

use axum::extract::State;
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use axum::routing::put;
use dotenv::dotenv;
use rand::prelude::*;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use ws2818_rgb_led_spi_driver::adapter_gen::WS28xxAdapter;
use ws2818_rgb_led_spi_driver::adapter_spi::WS28xxSpiAdapter;

use solar_status::SolarStatusDisplay;

use crate::error::SolarMonitorError;
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

async fn main_loop(
    mut powerwall: PowerwallApi,
    display: Box<&mut dyn SolarStatusDisplay>,
    shutdown_signal: Arc<AtomicBool>,
) -> Result<(), SolarMonitorError> {
    powerwall.wait_for_connection().await?;

    loop {
        let status = powerwall.get_stats().await?;

        display.show_status(status)?;

        thread::sleep(time::Duration::from_millis(1_000));

        if shutdown_signal.load(Ordering::Relaxed) {
            break;
        }
    }

    Ok(())
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn start_display(State(app_state): State<AppState>) -> impl IntoResponse {
    app_state.command_sender.send(Command::START).await.unwrap();

    println!("Starting solar monitor");

    "Starting solar monitor..."
}

async fn stop_display(State(app_state): State<AppState>) -> impl IntoResponse {
    app_state.command_sender.send(Command::STOP).await.unwrap();

    println!("Stopping solar monitor");

    "Stopping solar monitor..."
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

#[tokio::main]
async fn main() /* -> Result<(), Box<dyn std::error::Error + Send + Sync>>*/
{
    dotenv().ok();

    let (tx, mut rx) = mpsc::channel(32);

    let webserver_tx = tx.clone();
    let shutdown_tx = tx.clone();

    let ticker = tokio::spawn(async move {
        loop {
            tx.send(Command::TICK).await;
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

    #[cfg(not(feature = "i2c_display"))]
    let mut display = console_display::ConsoleDisplay {};

    // display.startup()?;

    // This address is localhost

    tokio::spawn(async move {
        #[cfg(feature = "i2c_display")]
        // let mut display = i2c_display::RaspiWithDisplay::new();
        let adapter = WS28xxSpiAdapter::new("/dev/spidev0.0").unwrap();
        let seven_segment_display = SevenSegmentDisplayString::new(adapter, 8);
        let mut display = rgbdigit_display::RgbDigitDisplay {
            display: &seven_segment_display,
            solar_generation_status: &mut seven_segment_display.derive_numeric_display(&[4, 5]),
            house_consumption_status: &mut seven_segment_display.derive_numeric_display(&[6, 7]),
            battery_status: &mut seven_segment_display.derive_numeric_display(&[0, 1]),
            grid_status: &mut seven_segment_display.derive_numeric_display(&[2, 3]),
            gradient: colorgrad::viridis(),
        };

        // display.startup().unwrap(); // @todo

        let mut powerwall = PowerwallApi::new().unwrap(); // @todo

        // powerwall.wait_for_connection().await.unwrap(); // @todo

        let mut output = false;

        while let Some(message) = rx.recv().await {
            let result = match message {
                Command::START => {
                    output = true;
                    Ok(())
                }
                Command::TICK => {
                    if output {
                        let status = powerwall.get_stats().await.unwrap(); // @todo

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
    });

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
        .with_graceful_shutdown(ctrl_c)
        .await
        .unwrap()
}
