use crate::error::SolarMonitorError;
use crate::rgbdigit::{NumericDisplay, SevenSegmentDisplayString};
use crate::solar_status::{SolarStatus, SolarStatusDisplay};
use ws2818_rgb_led_spi_driver::adapter_spi::WS28xxSpiAdapter;

pub struct RgbDigitDisplay {
    display: SevenSegmentDisplayString,
}

impl RgbDigitDisplay {
    pub(crate) fn new() -> RgbDigitDisplay {
        let adapter = WS28xxSpiAdapter::new("/dev/spidev0.0").unwrap();

        let display = SevenSegmentDisplayString::new(adapter, 4);

        RgbDigitDisplay { display }
    }
}

impl SolarStatusDisplay for RgbDigitDisplay {
    fn show_status(&mut self, status: SolarStatus) -> Result<(), SolarMonitorError> {
        let solar_generation_kw: f32 = status.solar_power_watts as f32 / 1000.0;
        let solar_generation_formatted = format!("{solar_generation_kw:.1}");

        // @todo it is insane to derive this display every single time we show status, it should be on the RgbDigitDisplay struct but I can't work out how!
        let mut solar_generation_status = self.display.derive_numeric_display(&[0, 1]);

        solar_generation_status.set_value(solar_generation_formatted);
        solar_generation_status.set_color((255, 255, 0));
        solar_generation_status.write();

        let house_consumption_kw: f32 = status.house_power_watts as f32 / 1000.0;
        let house_consumption_formatted = format!("{house_consumption_kw:.1}");
        let mut house_consumption_status = self.display.derive_numeric_display(&[2, 3]);
        house_consumption_status.set_value(house_consumption_formatted);
        house_consumption_status.set_color((0, 0, 255));
        house_consumption_status.write();

        self.display.flush();

        println!("{:?}", status);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), SolarMonitorError> {
        println!("Shutting down display");
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
