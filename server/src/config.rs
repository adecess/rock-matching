use crate::types::{MakerBotConfig, TakerBotConfig};
use axum::http::Uri;
use rock_matching_engine::{Price, Qty};
use std::env;
use std::net::SocketAddr;

pub(crate) struct AppConfig {
    pub(crate) bind_address: SocketAddr,
    pub(crate) allowed_origins: Vec<String>,
    pub(crate) event_channel_capacity: usize,
    pub(crate) command_channel_capacity: usize,
    pub(crate) max_websocket_connections: usize,
    pub(crate) maker: MakerBotConfig,
    pub(crate) taker: TakerBotConfig,
}

impl AppConfig {
    pub(crate) fn from_env() -> Result<Self, String> {
        let bind_address = env_setting("ROCK_BIND_ADDRESS")?;
        let allowed_origins = env_setting("ROCK_ALLOWED_ORIGINS")?;
        Self::with_runtime_settings(bind_address.as_deref(), allowed_origins.as_deref())
    }

    fn with_runtime_settings(
        bind_address: Option<&str>,
        allowed_origins: Option<&str>,
    ) -> Result<Self, String> {
        let mut config = Self::default();

        if let Some(bind_address) = bind_address {
            config.bind_address = bind_address
                .parse()
                .map_err(|error| format!("ROCK_BIND_ADDRESS must be a socket address: {error}"))?;
        }
        if let Some(allowed_origins) = allowed_origins {
            config.allowed_origins = parse_allowed_origins(allowed_origins)?;
        }

        Ok(config)
    }

    pub(crate) fn validate(self) -> Result<Self, &'static str> {
        if self.event_channel_capacity == 0 {
            return Err("event channel capacity must be greater than zero");
        }
        if self.command_channel_capacity == 0 {
            return Err("command channel capacity must be greater than zero");
        }
        if self.max_websocket_connections == 0 {
            return Err("maximum websocket connections must be greater than zero");
        }
        if self.allowed_origins.is_empty() {
            return Err("at least one websocket origin must be allowed");
        }

        Ok(self)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bind_address: SocketAddr::from(([0, 0, 0, 0], 3000)),
            allowed_origins: vec![
                "http://localhost:5173".to_owned(),
                "http://127.0.0.1:5173".to_owned(),
            ],
            event_channel_capacity: 16,
            command_channel_capacity: 100,
            max_websocket_connections: 100,
            maker: MakerBotConfig {
                reference_price: Price(100),
                max_bid_distance: Price(10),
                max_ask_distance: Price(10),
                max_quantity: Qty(10),
                min_quantity: Qty(1),
                delay_ms: 100,
            },
            taker: TakerBotConfig {
                max_quantity: Qty(10),
                min_quantity: Qty(1),
                delay_ms: 100,
                startup_delay_ms: 250,
            },
        }
    }
}

fn env_setting(name: &str) -> Result<Option<String>, String> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("{name}: {error}")),
    }
}

fn parse_allowed_origins(value: &str) -> Result<Vec<String>, String> {
    value
        .split(',')
        .map(str::trim)
        .map(|origin| {
            let uri = origin.parse::<Uri>().map_err(|_| {
                format!("ROCK_ALLOWED_ORIGINS contains an invalid origin: {origin}")
            })?;

            let valid_origin = match (uri.scheme_str(), uri.authority()) {
                (Some(scheme @ ("http" | "https")), Some(authority)) => {
                    uri.host().is_some()
                        && !authority.as_str().contains('@')
                        && origin == format!("{scheme}://{authority}")
                }
                _ => false,
            };

            if !valid_origin {
                return Err(format!(
                    "ROCK_ALLOWED_ORIGINS contains an invalid origin: {origin}"
                ));
            }

            Ok(origin.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_preserves_the_existing_runtime_values() {
        let config = AppConfig::default();

        assert_eq!(config.bind_address, SocketAddr::from(([0, 0, 0, 0], 3000)));
        assert_eq!(
            config.allowed_origins,
            ["http://localhost:5173", "http://127.0.0.1:5173"]
        );
        assert_eq!(config.event_channel_capacity, 16);
        assert_eq!(config.command_channel_capacity, 100);
        assert_eq!(config.max_websocket_connections, 100);

        assert_eq!(config.maker.reference_price, Price(100));
        assert_eq!(config.maker.max_bid_distance, Price(10));
        assert_eq!(config.maker.max_ask_distance, Price(10));
        assert_eq!(config.maker.max_quantity, Qty(10));
        assert_eq!(config.maker.min_quantity, Qty(1));
        assert_eq!(config.maker.delay_ms, 100);

        assert_eq!(config.taker.max_quantity, Qty(10));
        assert_eq!(config.taker.min_quantity, Qty(1));
        assert_eq!(config.taker.delay_ms, 100);
        assert_eq!(config.taker.startup_delay_ms, 250);
    }

    #[test]
    fn rejects_zero_capacities_and_limits() {
        let event_capacity_error = AppConfig {
            event_channel_capacity: 0,
            ..AppConfig::default()
        }
        .validate()
        .err();
        assert_eq!(
            event_capacity_error,
            Some("event channel capacity must be greater than zero")
        );

        let command_capacity_error = AppConfig {
            command_channel_capacity: 0,
            ..AppConfig::default()
        }
        .validate()
        .err();
        assert_eq!(
            command_capacity_error,
            Some("command channel capacity must be greater than zero")
        );

        let websocket_connection_limit_error = AppConfig {
            max_websocket_connections: 0,
            ..AppConfig::default()
        }
        .validate()
        .err();
        assert_eq!(
            websocket_connection_limit_error,
            Some("maximum websocket connections must be greater than zero")
        );
    }

    #[test]
    fn applies_runtime_settings_and_rejects_invalid_values() {
        let config = AppConfig::with_runtime_settings(
            Some("0.0.0.0:4000"),
            Some("https://app.example.com, https://preview.example.com"),
        )
        .unwrap();
        assert_eq!(config.bind_address, SocketAddr::from(([0, 0, 0, 0], 4000)));
        assert_eq!(
            config.allowed_origins,
            ["https://app.example.com", "https://preview.example.com"]
        );

        assert!(AppConfig::with_runtime_settings(Some("localhost:3000"), None).is_err());
        for origin in ["", "*", "wss://app.example.com", "https://app.example.com/"] {
            assert!(AppConfig::with_runtime_settings(None, Some(origin)).is_err());
        }
    }
}
