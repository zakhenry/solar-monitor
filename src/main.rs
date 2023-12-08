#[cfg(feature = "i2c_display")]
mod i2c_display;

mod solar_status;

#[cfg(not(feature = "i2c_display"))]
mod console_display;

mod error;
mod tesla_powerwall;

use std::{thread, time};

use crate::error::SolarMonitorError;
use crate::tesla_powerwall::PowerwallApi;
use dotenv::dotenv;
use solar_status::SolarStatusDisplay;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

#[tokio::main]
async fn main() -> Result<(), SolarMonitorError> {
    dotenv().ok();

    let powerwall = PowerwallApi::new()?;

    #[cfg(feature = "i2c_display")]
    let mut display = i2c_display::RaspiWithDisplay::new();

    let shutdown = Arc::new(AtomicBool::new(false));

    let shutdown_copy = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        println!("received ctrl+c");
        shutdown_copy.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    #[cfg(not(feature = "i2c_display"))]
    let mut display = console_display::ConsoleDisplay {};

    display.startup()?;

    let res = main_loop(powerwall, Box::new(&mut display), shutdown).await;

    if let Err(e) = res {
        display.show_error(&e)?;
        return Err(e);
    } else {
        display.shutdown()?;
    }
    Ok(())

}
