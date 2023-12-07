use crate::solar_status::{SolarStatus, SolarStatusDisplay};

#[cfg(not(feature = "i2c_display"))]
pub struct ConsoleDisplay;

#[cfg(not(feature = "i2c_display"))]
impl SolarStatusDisplay for ConsoleDisplay {
    fn show_status(&mut self, status: SolarStatus) {
        println!("{:?}", status)
    }
}
