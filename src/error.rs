use crate::tesla_powerwall::PowerwallApiError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum SolarMonitorError {
    DISPLAY(String),
    BITMAP(String),
    API(PowerwallApiError),
}

impl Display for SolarMonitorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::error::Error for SolarMonitorError {}
