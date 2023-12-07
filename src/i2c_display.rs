use embedded_graphics::{
    image::Image,
    mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use linux_embedded_hal::I2cdev;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use std::{thread, time};
use tinybmp::Bmp;
use tokio::time::Instant;

use crate::solar_status::{SolarStatus, SolarStatusDisplay};

pub struct RaspiWithDisplay {
    display:
        Ssd1306<I2CInterface<I2cdev>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>,
}

impl RaspiWithDisplay {
    pub fn new() -> RaspiWithDisplay {
        let i2c = I2cdev::new("/dev/i2c-1").unwrap();

        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().unwrap();

        RaspiWithDisplay { display }
    }
}

impl SolarStatusDisplay for RaspiWithDisplay {
    fn startup(&mut self) {
        let frames: Vec<Bmp<BinaryColor>> = vec![
            include_bytes!("resources/solar-spy-1.bmp"),
            include_bytes!("resources/solar-spy-2.bmp"),
            include_bytes!("resources/solar-spy-3.bmp"),
            include_bytes!("resources/solar-spy-4.bmp"),
            include_bytes!("resources/solar-spy-5.bmp"),
        ]
        .into_iter()
        .map(|it| Bmp::from_slice(it).unwrap())
        .collect();

        let duration = time::Duration::from_millis(2_000);

        let due_time = Instant::now() + duration;

        while due_time > Instant::now() {
            for frame in &frames {
                let img = Image::new(frame, Point::zero());
                img.draw(&mut self.display).unwrap();
                self.display.flush().unwrap();
                thread::sleep(time::Duration::from_millis(20));
            }
        }
    }

    fn show_status(&mut self, status: SolarStatus) {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let text_style_builder = TextStyleBuilder::new().baseline(Baseline::Top);
        let number_style = text_style_builder
            .clone()
            .alignment(Alignment::Right)
            .build();
        let text_style = text_style_builder.build();

        let left_align = 2;
        let row_spacing: i32 = (&character_style.font.character_size.height - 1) as i32;
        let right_align = 0;

        let rows = vec![
            ("Solar power", (status.solar_power_watts as f32) / 1000.0),
            ("Grid power", (status.grid_power_watts as f32) / 1000.0),
            ("House power", (status.house_power_watts as f32) / 1000.0),
            (
                "Battery power",
                (status.battery_power_watts as f32) / 1000.0,
            ),
        ];

        self.display.clear(BinaryColor::Off).unwrap();

        for (index, row) in rows.iter().enumerate() {
            let y_pos: i32 = &(index as i32) * &row_spacing - 1;

            Text::with_text_style(
                row.0,
                Point::new(left_align.to_owned(), y_pos),
                character_style,
                text_style,
            )
            .draw(&mut self.display)
            .unwrap();

            Text::with_text_style(
                &format!("{:.2}kW", row.1),
                Point::new(
                    (self.display.dimensions().0 - &right_align) as i32,
                    y_pos.to_owned(),
                ),
                character_style,
                number_style,
            )
            .draw(&mut self.display)
            .unwrap();
        }

        self.display.flush().unwrap();
    }

    fn shutdown(&mut self) {
        self.display.clear(BinaryColor::Off).unwrap();
        self.display.flush().unwrap();
    }
}
