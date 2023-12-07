#[derive(Debug)]
pub struct SolarStatus {
    pub solar_power_watts: i32,
    pub battery_power_watts: i32,
    pub house_power_watts: i32,
    pub grid_power_watts: i32,
}

pub trait SolarStatusDisplay {
    fn show_status(&mut self, status: SolarStatus);
    fn shutdown(&mut self);
    fn startup(&mut self);
}
