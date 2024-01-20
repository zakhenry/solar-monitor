use colorgrad::Gradient;

use crate::error::SolarMonitorError;
use crate::rgbdigit::{ NumericDisplay2, SevenSegmentDisplayString};
use crate::solar_status::{SolarStatus, SolarStatusDisplay};

pub struct RgbDigitDisplay2<'a> {
    pub(crate) display: &'a mut SevenSegmentDisplayString,
    pub(crate) solar_generation_status: NumericDisplay2,
    pub(crate) house_consumption_status: NumericDisplay2,
    pub(crate) battery_status: NumericDisplay2,
    pub(crate) grid_status: NumericDisplay2,
    pub(crate) gradient: Gradient,
}


impl SolarStatusDisplay for RgbDigitDisplay2<'_> {
    fn show_status(&mut self, status: SolarStatus) -> Result<(), SolarMonitorError> {
        let solar_generation_kw: f32 = status.solar_power_watts.clamp(0, i32::MAX) as f32 / 1000.0;
        let solar_generation_formatted = format!("{solar_generation_kw:.1}");

        &self
            .solar_generation_status
            .set_value(solar_generation_formatted);
        &self.solar_generation_status.set_color((100, 100, 0));
        &self.solar_generation_status.write();

        let house_consumption_kw: f32 = status.house_power_watts as f32 / 1000.0;
        let house_consumption_formatted = format!("{house_consumption_kw:.1}");
        &self
            .house_consumption_status
            .set_value(house_consumption_formatted);
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

