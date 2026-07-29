use crate::types::{MakerBotConfig, TakerBotConfig};
use rock_matching_engine::{Price, Qty};
use std::net::SocketAddr;

pub(crate) struct AppConfig {
    pub(crate) bind_address: SocketAddr,
    pub(crate) event_channel_capacity: usize,
    pub(crate) command_channel_capacity: usize,
    pub(crate) maker: MakerBotConfig,
    pub(crate) taker: TakerBotConfig,
}

impl AppConfig {
    pub(crate) fn validate(self) -> Result<Self, &'static str> {
        if self.event_channel_capacity == 0 {
            return Err("event channel capacity must be greater than zero");
        }
        if self.command_channel_capacity == 0 {
            return Err("command channel capacity must be greater than zero");
        }

        Ok(self)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bind_address: SocketAddr::from(([0, 0, 0, 0], 3000)),
            event_channel_capacity: 16,
            command_channel_capacity: 100,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_preserves_the_existing_runtime_values() {
        let config = AppConfig::default();

        assert_eq!(config.bind_address, SocketAddr::from(([0, 0, 0, 0], 3000)));
        assert_eq!(config.event_channel_capacity, 16);
        assert_eq!(config.command_channel_capacity, 100);

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
    fn rejects_zero_channel_capacities() {
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
    }
}
