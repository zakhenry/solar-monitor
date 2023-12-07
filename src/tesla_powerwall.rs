use crate::solar_status::SolarStatus;

use std::collections::HashMap;
use std::{thread, time};

extern crate reqwest_rustls_tls as reqwest;

use rand::Rng;
use serde::Deserialize;
use std::env;
use std::fmt::{Debug, Display, Formatter};

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
    battery: Site,
    load: Load,
    solar: Solar,
}

impl From<MetersAggregatesResponse> for SolarStatus {
    fn from(value: MetersAggregatesResponse) -> Self {
        SolarStatus {
            solar_power_watts: value.solar.instant_power as i32,
            battery_power_watts: value.battery.instant_power as i32,
            house_power_watts: value.load.instant_power as i32,
            grid_power_watts: value.site.instant_power as i32,
        }
    }
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

    async fn get_token(&mut self) -> Result<String, PowerwallApiError> {
        if let Some(token) = &self.api_token {
            return Ok(token.to_owned());
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

    pub async fn get_stats(&mut self) -> Result<SolarStatus, PowerwallApiError> {
        let body: MetersAggregatesResponse = self
            .client
            .get(format!("https://{}/api/meters/aggregates", self.ip_address))
            .bearer_auth(self.get_token().await?)
            .send()
            .await?
            .json::<MetersAggregatesResponse>()
            .await?;

        Ok(body.into())
    }
}