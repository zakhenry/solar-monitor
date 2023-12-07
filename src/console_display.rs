use crate::solar_status::{SolarStatus, SolarStatusDisplay};

pub struct ConsoleDisplay;

impl SolarStatusDisplay for ConsoleDisplay {
    fn show_status(&mut self, status: SolarStatus) {
        println!("{:?}", status)
    }

    fn shutdown(&mut self) {
        println!("Shutting down display");
    }

    fn startup(&mut self) {
        println!("Starting display");
    }
}
