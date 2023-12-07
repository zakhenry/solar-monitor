#[cfg(feature = "i2c_display")]
mod i2c_display;

mod solar_status;

#[cfg(not(feature = "i2c_display"))]
mod console_display;

mod tesla_powerwall;

use std::{thread, time};

use dotenv::dotenv;
use solar_status::{SolarStatus, SolarStatusDisplay};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // @todo use application specific errors
    dotenv().ok();

    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }

    let mut powerwall = tesla_powerwall::PowerwallApi::new()?;

    #[cfg(feature = "i2c_display")]
    let mut display = i2c_display::RaspiWithDisplay::new();

    #[cfg(not(feature = "i2c_display"))]
    let mut display = console_display::ConsoleDisplay {};

    loop {
        let status = powerwall.get_stats().await?;

        display.show_status(status);

        thread::sleep(time::Duration::from_millis(1_000));
    }
}
