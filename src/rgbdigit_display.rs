use std::cell::RefCell;
use colorgrad::Gradient;
use crate::error::SolarMonitorError;
use crate::rgbdigit::{NumericDisplay, SevenSegmentDisplayString};
use crate::solar_status::{SolarStatus, SolarStatusDisplay};

pub struct RgbDigitDisplay<'a> {
    pub(crate) display: &'a SevenSegmentDisplayString,
    pub(crate) solar_generation_status: &'a mut NumericDisplay<'a>,
    pub(crate) house_consumption_status: &'a mut NumericDisplay<'a>,
    pub(crate) battery_status: &'a mut NumericDisplay<'a>,
    pub(crate) grid_status: &'a mut NumericDisplay<'a>,
    pub(crate) gradient: Gradient,
}

impl RgbDigitDisplay<'_> {
    // this doesn't work. @todo understand why and refactor the construction logic our of main.rs
    // fn new<'a>() -> RgbDigitDisplay<'a> {
    //
    //     let adapter = WS28xxSpiAdapter::new("/dev/spidev0.0").unwrap();
    //     let seven_segment_display = SevenSegmentDisplayString::new(adapter, 4);
    //
    //     RgbDigitDisplay {
    //         display: &seven_segment_display,
    //         solar_generation_status: &mut seven_segment_display.derive_numeric_display(&[0, 1]),
    //         house_consumption_status: &mut seven_segment_display.derive_numeric_display(&[2, 3]),
    //     }
    // }
}

unsafe impl Send for RgbDigitDisplay<'_> {}
unsafe impl Sync for RgbDigitDisplay<'_> {}
unsafe impl Send for SevenSegmentDisplayString {}
unsafe impl Sync for SevenSegmentDisplayString {}
unsafe impl Send for NumericDisplay<'_> {}
unsafe impl Sync for NumericDisplay<'_> {}

impl SolarStatusDisplay for RgbDigitDisplay<'_> {


    fn show_status(&mut self, status: SolarStatus) -> Result<(), SolarMonitorError> {
        let solar_generation_kw: f32 = status.solar_power_watts.clamp(0, i32::MAX) as f32 / 1000.0;
        let solar_generation_formatted = format!("{solar_generation_kw:.1}");

        &self.solar_generation_status.set_value(solar_generation_formatted);
        &self.solar_generation_status.set_color((100, 100, 0));
        &self.solar_generation_status.write();

        let house_consumption_kw: f32 = status.house_power_watts as f32 / 1000.0;
        let house_consumption_formatted = format!("{house_consumption_kw:.1}");
        &self.house_consumption_status.set_value(house_consumption_formatted);
        &self.house_consumption_status.set_color((30, 10, 80));
        &self.house_consumption_status.write();

        let battery_kw: f32 = (status.battery_power_watts as f32 / 1000.0).abs();
        let battery_formatted = format!("{battery_kw:.1}");
        &self.battery_status.set_value(battery_formatted);
        if battery_kw > -0.1 {
            &self.battery_status.set_color((30, 70, 20));
        } else {
            &self.battery_status.set_color((100, 40, 10));
        }
        &self.battery_status.write();

        let grid_kw: f32 = (status.grid_power_watts as f32 / 1000.0).abs();
        let grid_formatted = format!("{grid_kw:.1}");
        &self.grid_status.set_value(grid_formatted);
        if grid_kw > -0.1 {
            &self.grid_status.set_color((30, 30, 30));
        } else {
            &self.grid_status.set_color((50, 0, 0));
        }
        &self.grid_status.write();

        self.display.flush();

        println!("{:?}", status);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), SolarMonitorError> {
        println!("Shutting down display");
        &self.solar_generation_status.clear();
        &self.solar_generation_status.write();
        &self.house_consumption_status.clear();
        &self.house_consumption_status.write();
        &self.battery_status.clear();
        &self.battery_status.write();
        &self.grid_status.clear();
        &self.grid_status.write();
        self.display.flush();
        Ok(())
    }

    fn startup(&mut self) -> Result<(), SolarMonitorError> {
        println!("Starting display");
        Ok(())
    }

    fn show_error(&mut self, err: &SolarMonitorError) -> Result<(), SolarMonitorError> {
        eprintln!("Intercepted error: {:?}", err);
        Ok(())
    }
}
