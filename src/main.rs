#[cfg(feature = "i2c_display")]
mod i2c_display;

mod solar_status;

#[cfg(not(feature = "i2c_display"))]
mod console_display;

mod error;
mod tesla_powerwall;

use std::{thread, time};
use std::cell::RefCell;

use crate::error::SolarMonitorError;
use crate::tesla_powerwall::PowerwallApi;
use dotenv::dotenv;
use rand::prelude::*;
use solar_status::SolarStatusDisplay;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ws2818_rgb_led_spi_driver::adapter_gen::WS28xxAdapter;
use ws2818_rgb_led_spi_driver::adapter_spi::WS28xxSpiAdapter;
use ws2818_rgb_led_spi_driver::encoding::encode_rgb;

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

const ZERO: u8 = 0b00111111;
const ONE: u8 = 0b00000110;
const TWO: u8 = 0b01011011;
const THREE: u8 = 0b01001111;
const FOUR: u8 = 0b01100110;
const FIVE: u8 = 0b01101101;
const SIX: u8 = 0b01111101;
const SEVEN: u8 = 0b00000111;
const EIGHT: u8 = 0b01111111;
const NINE: u8 = 0b01101111;
//                     87654321
fn main() {
    let mut adapter = WS28xxSpiAdapter::new("/dev/spidev0.0").unwrap();

    let mut display_string = SevenSegmentDisplayString::new(&mut adapter, 4);

    // println!("Running loop");
    //
    // loop {
    //     for i in 0..=9 {
    //         let random_rgb = (random::<u8>(), random::<u8>(), random::<u8>());
    //         display_string.set_digit(0, i, random_rgb);
    //         for j in 0..=9 {
    //             display_string.set_digit(1, j, random_rgb);
    //
    //             display_string.flush();
    //
    //             thread::sleep(time::Duration::from_millis(100));
    //         }
    //     }
    // }
    {
        display_string.set_digit(0, 1, (105,68,5));
        display_string.set_digit(1, 2, (43,57,6));
        display_string.set_digit(2, 3, (107,6,6));
        display_string.set_digit(3, 4, (40,5,45));
        // let mut number_field = display_string.derive_numeric_display(&[0, 1]);
        //
        // number_field.set_raw(Some(42), None, (123, 0, 234));
    }
    display_string.flush();

    println!("done!");

}

trait WriteRgbDigit {
    fn write_spi_encoded(&mut self, encoded: &Vec<u8>) -> Result<(), String>;
}

impl WriteRgbDigit for WS28xxSpiAdapter {
    fn write_spi_encoded(&mut self, encoded: &Vec<u8>) -> Result<(), String> {
        let encoded: Vec<u8> = encoded
            .chunks(3)
            .flat_map(|chunk| {
                let [r, g, b]: [_; 3] = chunk
                    .try_into()
                    .expect("should be chunks of three for each r/g/b channel!");
                encode_rgb(r, g, b)
            })
            .collect();

        self.write_encoded_rgb(&encoded)
    }
}

#[derive(Clone)]
struct SevenSegmentDisplay {
    state_rgb: [u8; 24],
}

struct SevenSegmentDisplayString<'a> {
    digits: Vec<SevenSegmentDisplay>,
    adapter: &'a mut dyn WriteRgbDigit,
}

impl <'a>SevenSegmentDisplayString<'a> {
    fn new(adapter: &mut impl WriteRgbDigit, display_count: usize) -> SevenSegmentDisplayString {

        let display_init: [u8; 24] = random();
        let new_display_state = SevenSegmentDisplay {
            state_rgb: display_init,
        };

        let digits = vec![new_display_state; display_count];

        return SevenSegmentDisplayString {
            digits,
            adapter,
        };
    }

    fn flush(&mut self) {
        let encoded: Vec<u8> = self.digits.iter().flat_map(|it| it.state_rgb).collect();

        self.adapter
            .write_spi_encoded(&encoded)
            .expect("should work");
    }

    fn derive_numeric_display(&'a mut self, display_indices: &'a[usize]) -> NumericDisplay<'a> {

        return NumericDisplay {
            display_indices,
            display_string: self,
            value_raw: None,
            decimal: None,
            color_rgb: (0, 0, 0)
        }

    }

    fn set_digit(&mut self, digit_index: usize, value: u8, color: (u8, u8, u8)) {
        let encoded = match value {
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
            _ => panic!("Single digits only allowed!"),
        };

        let mut led_colors: [u8; 24] = [0; 24];
        let (r, g, b) = color;
        let color_slice = [r, g, b];

        for i in 0..8 {
            if encoded >> i & 1 == 1 {
                let offset = i * 3;

                led_colors[offset..offset + 3].copy_from_slice(&color_slice);
            }
        }

        self.digits[digit_index].state_rgb = led_colors;
    }
}

struct NumericDisplay<'a> {
    display_indices: &'a[usize],
    display_string: & 'a mut SevenSegmentDisplayString<'a>,
    value_raw: Option<i32>,
    decimal: Option<u8>,
    color_rgb: (u8, u8, u8),
}

impl NumericDisplay<'_> {

    fn clear(&mut self) {
        self.value_raw = None
    }

    fn set_raw(&mut self, value_raw: Option<i32>, decimal: Option<u8>, color_rgb: (u8, u8, u8)) {
        self.color_rgb = color_rgb;
        self.value_raw = value_raw;
        self.decimal = decimal;
    }

    fn write(&mut self) {

        if self.value_raw == None {
            return
        }

        let mut num = self.value_raw.unwrap();
        let base = 10;
        let mut power = 0;
        while num != 0 {
            let digit = num % base;

            self.display_string.set_digit(self.display_indices[power], digit.try_into().expect("hmm"), self.color_rgb);

            num /= base;
            power += 1;
        }

    }

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
