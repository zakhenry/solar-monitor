#[cfg(feature = "i2c_display")]
mod i2c_display;

mod solar_status;

#[cfg(not(feature = "i2c_display"))]
mod console_display;

mod tesla_powerwall;

use std::{thread, time};

use dotenv::dotenv;
use solar_status::SolarStatusDisplay;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    let mut powerwall = tesla_powerwall::PowerwallApi::new()?;

    #[cfg(feature = "i2c_display")]
    let mut display = i2c_display::RaspiWithDisplay::new();

    let shutdown = Arc::new(AtomicBool::new(false));

    let shutdown_copy = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        shutdown_copy.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    #[cfg(not(feature = "i2c_display"))]
    let mut display = console_display::ConsoleDisplay {};

    display.startup();

    loop {
        let status = powerwall.get_stats().await?;

        display.show_status(status);

        thread::sleep(time::Duration::from_millis(1_000));

        if shutdown.load(Ordering::Relaxed) {
            break;
        }
    }

    display.shutdown();

    Ok(())
}
