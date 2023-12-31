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
use ws2818_rgb_led_spi_driver::adapter_gen::WS28xxAdapter;
use ws2818_rgb_led_spi_driver::adapter_spi::WS28xxSpiAdapter;
use ws2818_rgb_led_spi_driver::encoding::encode_rgb;
use rand::prelude::*;

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

// #[tokio::main]
// async fn main() -> Result<(), SolarMonitorError> {
//     dotenv().ok();
//
//     let powerwall = PowerwallApi::new()?;
//
//     #[cfg(feature = "i2c_display")]
//     let mut display = i2c_display::RaspiWithDisplay::new();
//
//     let shutdown = Arc::new(AtomicBool::new(false));
//
//     let shutdown_copy = Arc::clone(&shutdown);
//     ctrlc::set_handler(move || {
//         println!("received ctrl+c");
//         shutdown_copy.store(true, Ordering::Relaxed);
//     })
//     .expect("Error setting Ctrl-C handler");
//
//     #[cfg(not(feature = "i2c_display"))]
//     let mut display = console_display::ConsoleDisplay {};
//
//     display.startup()?;
//
//     let res = main_loop(powerwall, Box::new(&mut display), shutdown).await;
//
//     if let Err(e) = res {
//         display.show_error(&e)?;
//         return Err(e);
//     } else {
//         display.shutdown()?;
//     }
//     Ok(())
// }


const ZERO: i32     = 0b00111111;
const ONE: i32      = 0b00000110;
const TWO: i32      = 0b01011011;
const THREE: i32    = 0b01001111;
const FOUR: i32     = 0b01100110;
const FIVE: i32     = 0b01101101;
const SIX: i32      = 0b01111101;
const SEVEN: i32    = 0b00000111;
const EIGHT: i32    = 0b01111111;
const NINE: i32     = 0b01101111;
//                      87654321
fn main() {

    let mut adapter = WS28xxSpiAdapter::new("/dev/spidev0.0").unwrap();

    loop {

        for i in 0..=9 {


            let random_rgb = (rand::random::<u8>(), rand::random::<u8>(), rand::random::<u8>());

            write_digit(i, &mut adapter, random_rgb);

            thread::sleep(time::Duration::from_millis(200));
        }

    }
}

fn write_digit(digit: u8, adapter: &mut WS28xxSpiAdapter, color: (u8, u8, u8)) {

    let encoded = match digit {
        0 => ZERO,
        1 => ONE,
        2 => TWO,
        3 => THREE,
        4 => FOUR,
        5 => FIVE,
        6 => SIX,
        7 => SEVEN,
        8 => EIGHT,
        9 => NINE,
        _ => panic!("Single digits only allowed!")
    };

    let mut spi_encoded_rgb_bits = vec![];

    for i in 0..8 {

        if encoded >> i & 1 == 1 {
            let (r, g, b) = color;
            spi_encoded_rgb_bits.extend_from_slice(&encode_rgb(r, g, b));
        } else {
            spi_encoded_rgb_bits.extend_from_slice(&encode_rgb(0, 0, 0));
        }

    }

    adapter.write_encoded_rgb(&spi_encoded_rgb_bits).unwrap();

}


/*

Digit layout:

 1111
6    2
6    2
 7777
5    3
5    3
 4444   8

*/
