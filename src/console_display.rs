use crate::error::SolarMonitorError;
use crate::solar_status::{SolarStatus, SolarStatusDisplay};

pub struct ConsoleDisplay;

impl SolarStatusDisplay for ConsoleDisplay {
    fn show_status(&mut self, status: SolarStatus) -> Result<(), SolarMonitorError> {
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
