use crate::error::SolarMonitorError;
#[derive(Debug)]
pub struct SolarStatus {
    pub solar_power_watts: i32,
    pub battery_power_watts: i32,
    pub house_power_watts: i32,
    pub grid_power_watts: i32,
    pub battery_level_percent: f64,
}

pub trait SolarStatusDisplay {
    fn show_status(&mut self, status: SolarStatus) -> Result<(), SolarMonitorError>;
    fn shutdown(&mut self) -> Result<(), SolarMonitorError>;
    fn startup(&mut self) -> Result<(), SolarMonitorError>;
    fn clear(&mut self) -> Result<(), SolarMonitorError>;
    fn show_error(&mut self, err: &SolarMonitorError) -> Result<(), SolarMonitorError>;
}
