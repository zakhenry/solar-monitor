#[cfg(feature = "i2c_display")]
mod i2c_display;

mod solar_status;

#[cfg(not(feature = "i2c_display"))]
mod console_display;

use rand::Rng;
use std::{thread, time}; // 0.8.5

use solar_status::{SolarStatus, SolarStatusDisplay};

fn main() {
    println!("Hello, world!");

    #[cfg(feature = "i2c_display")]
    let mut display = i2c_display::RaspiWithDisplay::new();

    #[cfg(not(feature = "i2c_display"))]
    let mut display = console_display::ConsoleDisplay {};

    loop {
        let status = SolarStatus {
            solar_power_watts: rand::thread_rng().gen_range(0..10_000),
            battery_power_watts: rand::thread_rng().gen_range(-5_000..5_000),
            house_power_watts: rand::thread_rng().gen_range(0..15_000),
            grid_power_watts: rand::thread_rng().gen_range(-5000..15_000),
        };

        display.show_status(status);

        thread::sleep(time::Duration::from_millis(1_000));
    }
}
