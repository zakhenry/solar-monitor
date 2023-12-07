use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use linux_embedded_hal::I2cdev;
use core::fmt::Write;
use std::{thread, time};
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::text::Baseline::{Middle, Top};
use embedded_graphics::text::{Alignment, TextStyleBuilder};

struct SolarStatus {
    solar_power_watts: i32,
    battery_power_watts: i32,
    house_power_watts: i32,
    grid_power_watts: i32
}


trait DisplaySolarStatus {
    fn show_status(&mut self, status: SolarStatus);
}

// fn format(kw)

impl DisplaySolarStatus for Ssd1306<I2CInterface<I2cdev>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>> {
    fn show_status(&mut self, status: SolarStatus) {

        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let text_style_builder = TextStyleBuilder::new().baseline(Top);
        let number_style = text_style_builder.clone().alignment(Alignment::Right).build();
        let text_style = text_style_builder.build();

        let left_align = 2;
        let row_spacing: i32 = (&character_style.font.character_size.height - 1 ) as i32;
        let right_align = 0;

        println!("{:?}", self.dimensions());

        let rows = vec![
            ("Solar power", (status.solar_power_watts as f32) / 1000.0),
            ("Grid power", (status.grid_power_watts as f32) / 1000.0),
            ("House power", (status.house_power_watts as f32) / 1000.0),
            ("Battery power", (status.battery_power_watts as f32) / 1000.0),
        ];

        for (index, row) in rows.iter().enumerate() {
            let y_pos: i32 = &(index as i32) * &row_spacing - 1;

            Text::with_text_style(row.0, Point::new(left_align.to_owned(), y_pos), character_style, text_style).draw(self).unwrap();
            Text::with_text_style(&format!("{:.2}kW", row.1), Point::new((self.dimensions().0 - &right_align) as i32, y_pos.to_owned()), character_style, number_style).draw(self).unwrap();
        }

        self.flush().unwrap();
    }
}

fn main() {

    println!("Hello, world!");

    let i2c = I2cdev::new("/dev/i2c-1").unwrap();

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(
    interface,
    DisplaySize128x32,
    DisplayRotation::Rotate0,
    ).into_buffered_graphics_mode();
    display.init().unwrap();

    let status = SolarStatus {
        solar_power_watts: 9089,
        battery_power_watts: -160,
        house_power_watts: 131,
        grid_power_watts: -8780
    };

    display.show_status(status);

    // let text_style = MonoTextStyleBuilder::new()
    // .font(&FONT_8X13)
    // .text_color(BinaryColor::On)
    // .build();
    //
    // display.clear(BinaryColor::On).unwrap();
    // display.flush().unwrap();
    // display.clear(BinaryColor::Off).unwrap();
    // display.flush().unwrap();
    //
    // Text::with_baseline("Hello Louise!", Point::zero(), text_style, Baseline::Top)
    // .draw(&mut display)
    // .unwrap();
    // display.flush().unwrap();
    //
    // thread::sleep(time::Duration::from_millis(100));
    //
    //
    // Text::with_baseline("Hello Ada!", Point::new(0, 12), text_style, Baseline::Middle)
    // .draw(&mut display)
    // .unwrap();
    //
    // display.flush().unwrap();
    //
    // thread::sleep(time::Duration::from_millis(1_000));
    // display.clear(BinaryColor::Off).unwrap();
    //
    // display.flush().unwrap();

}
