extern crate reqwest_rustls_tls as reqwest;

use std::collections::HashMap;
use std::env;
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;

use reqwest_rustls_tls::{Error, Response};
use serde::Deserialize;

use crate::error::SolarMonitorError;
use crate::solar_status::SolarStatus;

pub struct PowerwallApi {
    ip_address: String,
    api_token: Option<String>,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

#[derive(Deserialize)]
struct Site {
    instant_power: f64,
}

#[derive(Deserialize)]
struct Battery {
    instant_power: f64,
}
#[derive(Deserialize)]
struct Load {
    instant_power: f64,
}
#[derive(Deserialize)]
struct Solar {
    instant_power: f64,
}

#[derive(Deserialize)]
struct MetersAggregatesResponse {
    site: Site,
    battery: Battery,
    load: Load,
    solar: Solar,
}

impl From<(MetersAggregatesResponse, BatteryLevelResponse)> for SolarStatus {
    fn from(
        (meter_aggregates, battery_level): (MetersAggregatesResponse, BatteryLevelResponse),
    ) -> Self {
        SolarStatus {
            solar_power_watts: meter_aggregates.solar.instant_power as i32,
            battery_power_watts: meter_aggregates.battery.instant_power as i32,
            house_power_watts: meter_aggregates.load.instant_power as i32,
            grid_power_watts: meter_aggregates.site.instant_power as i32,
            // Note: Tesla App reserves 5% of battery = ( (batterylevel / 0.95) - (5 / 0.95) )
            battery_level_percent: (battery_level.percentage / 0.95) - (5.0 / 0.95),
        }
    }
}

#[derive(Deserialize)]
struct BatteryLevelResponse {
    percentage: f64,
}

#[derive(Debug)]
pub enum PowerwallApiError {
    Env(env::VarError),
    Request(reqwest::Error),
}

impl From<env::VarError> for PowerwallApiError {
    fn from(value: env::VarError) -> Self {
        PowerwallApiError::Env(value)
    }
}

impl From<reqwest::Error> for PowerwallApiError {
    fn from(value: reqwest::Error) -> Self {
        PowerwallApiError::Request(value)
    }
}

impl Display for PowerwallApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<PowerwallApiError> for SolarMonitorError {
    fn from(value: PowerwallApiError) -> Self {
        SolarMonitorError::API(value)
    }
}

impl std::error::Error for PowerwallApiError {}

impl PowerwallApi {
    pub fn new() -> Result<PowerwallApi, PowerwallApiError> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();

        Ok(PowerwallApi {
            ip_address: env::var("POWERWALL_API_ADDRESS")?,
            api_token: None,
            client,
        })
    }

    async fn check_status(&self) -> Result<Response, Error> {
        self.client
            .get(format!("https://{}/api/status", self.ip_address))
            .send()
            .await
    }

    pub async fn wait_for_connection(&self) -> Result<(), PowerwallApiError> {
        println!("Checking connection");

        tryhard::retry_fn(|| self.check_status())
            .retries(300) // 1 full minute
            .fixed_backoff(Duration::from_millis(200))
            .await?;

        println!("Connection is ready");

        Ok(())
    }

    async fn get_token(&mut self, force: bool) -> Result<String, PowerwallApiError> {
        if !force {
            if let Some(token) = &self.api_token {
                return Ok(token.to_owned());
            }
        }

        let mut request_body = HashMap::new();

        let password = env::var("POWERWALL_PASSWORD")?;

        request_body.insert("username", "customer");
        request_body.insert("email", "");
        request_body.insert("password", &*password);

        let response = self
            .client
            .post(format!("https://{}/api/login/Basic", &self.ip_address))
            .json(&request_body)
            .send()
            .await?;

        println!("Request responded with status {}", response.status());
        assert!(response.status().is_success());

        let body = response.json::<LoginResponse>().await?;

        self.api_token = Some(body.token.clone());

        println!("Loaded token {:?}", self.api_token);

        Ok(body.token.clone())
    }

    async fn get_stats_response(&mut self) -> Result<reqwest::Response, PowerwallApiError> {
        Ok(self
            .client
            .get(format!("https://{}/api/meters/aggregates", self.ip_address))
            .bearer_auth(self.get_token(false).await?)
            .send()
            .await?)
    }

    async fn get_battery_percentage_response(
        &mut self,
    ) -> Result<reqwest::Response, PowerwallApiError> {
        Ok(self
            .client
            .get(format!("https://{}/api/system_status/soe", self.ip_address))
            .bearer_auth(self.get_token(false).await?)
            .send()
            .await?)
    }

    async fn get_meter_aggregates(
        &mut self,
    ) -> Result<MetersAggregatesResponse, PowerwallApiError> {
        let response = self.get_stats_response().await?;

        let body = match response.status() {
            reqwest::StatusCode::OK => response,
            reqwest::StatusCode::UNAUTHORIZED => {
                println!("Token became invalid, fetching another one");
                self.get_token(true).await?;
                self.get_stats_response().await?
            }
            status => panic!("Unhandled status code {}", status),
        }
        .json::<MetersAggregatesResponse>()
        .await?;

        Ok(body)
    }

    async fn get_battery_percentage(&mut self) -> Result<BatteryLevelResponse, PowerwallApiError> {
        let response = self.get_battery_percentage_response().await?;

        let body = match response.status() {
            reqwest::StatusCode::OK => response,
            reqwest::StatusCode::UNAUTHORIZED => {
                println!("Token became invalid, fetching another one");
                self.get_token(true).await?;
                self.get_stats_response().await?
            }
            status => panic!("Unhandled status code {}", status),
        }
        .json::<BatteryLevelResponse>()
        .await?;

        Ok(body)
    }

    pub async fn get_stats(&mut self) -> Result<SolarStatus, PowerwallApiError> {
        // @todo rewrite to run these concurrently. Will require changing the token to refcell so it can be borrowed mutably concurrently (or with mutex + arc or something)
        let meter_aggregates = self.get_meter_aggregates().await?;
        let battery_response = self.get_battery_percentage().await?;

        Ok((meter_aggregates, battery_response).into())
    }
}
