use crate::error::SolarMonitorError;
use crate::rgbdigit::{NumericDisplay, SevenSegmentChar, SevenSegmentDisplayString};
use crate::solar_status::{SolarStatus, SolarStatusDisplay};
use std::time::Duration;

pub struct RgbDigitDisplay<'a> {
    pub(crate) display: &'a SevenSegmentDisplayString,
    pub(crate) solar_generation_status: &'a mut NumericDisplay<'a>,
    pub(crate) house_consumption_status: &'a mut NumericDisplay<'a>,
    pub(crate) battery_status: &'a mut NumericDisplay<'a>,
    pub(crate) grid_status: &'a mut NumericDisplay<'a>,
    pub(crate) battery_level: &'a mut NumericDisplay<'a>,
}

impl From<String> for SolarMonitorError {
    fn from(value: String) -> Self {
        Self::DISPLAY(value)
    }
}

impl RgbDigitDisplay<'_> {
    pub(crate) async fn start_await(&mut self) -> Result<(), SolarMonitorError> {
        loop {
            self.display
                .set_all(&SevenSegmentChar::BLANK, (100, 100, 100), true);
            self.display.flush();
            tokio::time::sleep(Duration::from_millis(30)).await;
            self.display
                .set_all(&SevenSegmentChar::BLANK, (0, 0, 0), false);
            self.display.flush();
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
    }
}

impl SolarStatusDisplay for RgbDigitDisplay<'_> {
    fn show_status(&mut self, status: SolarStatus) -> Result<(), SolarMonitorError> {
        let solar_generation_kw: f32 = status.solar_power_watts.clamp(0, i32::MAX) as f32 / 1000.0;
        let solar_generation_formatted = format!("{solar_generation_kw:.1}");

        self.solar_generation_status
            .set_value(solar_generation_formatted);
        self.solar_generation_status.set_color((100, 100, 0));
        self.solar_generation_status.write()?;
        let house_consumption_kw: f32 = status.house_power_watts as f32 / 1000.0;
        let house_consumption_formatted = format!("{house_consumption_kw:.1}");
        self.house_consumption_status
            .set_value(house_consumption_formatted);
        self.house_consumption_status.set_color((30, 10, 80));
        self.house_consumption_status.write()?;

        let battery_kw: f32 = (status.battery_power_watts as f32 / 1000.0).abs();
        let battery_formatted = format!("{battery_kw:.1}");
        self.battery_status.set_value(battery_formatted);
        if status.battery_power_watts > -100 {
            // epsilon to stop it from flickering while around zero
            self.battery_status.set_color((100, 40, 10));
        } else {
            self.battery_status.set_color((30, 70, 20));
        }
        self.battery_status.write()?;

        let grid_kw: f32 = (status.grid_power_watts as f32 / 1000.0).abs();
        let grid_formatted = format!("{grid_kw:.1}");
        self.grid_status.set_value(grid_formatted);
        if status.grid_power_watts > 100 {
            // epsilon to stop it from flickering while around zero
            self.grid_status.set_color((50, 0, 0));
        } else {
            self.grid_status.set_color((30, 30, 30));
        }
        self.grid_status.write()?;

        self.battery_level.set_value(format!(
            "{:.0}",
            // clamp as 100% requires 3 digits which we don't have
            status.battery_level_percent.clamp(0.0, 99.0)
        ));
        self.battery_level.set_color((100, 0, 100));
        self.battery_level.write()?;

        self.display.flush();

        println!("{:?}", status);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), SolarMonitorError> {
        println!("Shutting down display");
        self.clear()?;
        Ok(())
    }

    fn startup(&mut self) -> Result<(), SolarMonitorError> {
        println!("Starting display");

        self.display
            .set_all(&SevenSegmentChar::BLANK, (0, 0, 100), true);
        self.display.flush();

        Ok(())
    }

    fn clear(&mut self) -> Result<(), SolarMonitorError> {
        println!("Clearing display");

        self.display
            .set_all(&SevenSegmentChar::BLANK, (0, 0, 0), false);
        self.display.flush();

        Ok(())
    }

    fn show_error(&mut self, err: &SolarMonitorError) -> Result<(), SolarMonitorError> {
        self.display
            .set_all(&SevenSegmentChar::Char('E'), (255, 0, 0), false);
        self.display.flush();
        eprintln!("Intercepted error: {:?}", err);
        Ok(())
    }
}
