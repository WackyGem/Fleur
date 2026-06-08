//! Bollinger Bands daily indicator calculation.

use std::error::Error;
use std::fmt;

use crate::operators::RollingMeanStdDev;

/// Canonical Bollinger Bands configurations for the first production version.
pub const DEFAULT_BOLL_CONFIGS: [BollConfig; 3] = [
    BollConfig {
        window: 10,
        multiplier: 1.5,
        field_suffix: "10_1p5",
    },
    BollConfig {
        window: 20,
        multiplier: 2.0,
        field_suffix: "20_2",
    },
    BollConfig {
        window: 50,
        multiplier: 2.5,
        field_suffix: "50_2p5",
    },
];

/// Bollinger Bands use population standard deviation, equivalent to `ddof = 0`.
pub const DEFAULT_BOLL_STDDEV_DDOF: u8 = 0;

/// The largest canonical Bollinger Bands window.
pub const DEFAULT_BOLL_MAX_WINDOW: usize = 50;

/// Single security Bollinger Bands input row.
#[derive(Debug, Clone, PartialEq)]
pub struct BollInput {
    /// Trade date represented as an ISO-like sortable string.
    pub trade_date: String,
    /// Forward-adjusted close price.
    pub close_price: Option<f64>,
}

impl BollInput {
    /// Create a Bollinger Bands input row.
    pub fn new(trade_date: impl Into<String>, close_price: Option<f64>) -> Self {
        Self {
            trade_date: trade_date.into(),
            close_price,
        }
    }
}

/// A single Bollinger Bands parameter configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollConfig {
    /// Number of valid close values in the rolling window.
    pub window: usize,
    /// Standard deviation multiplier.
    pub multiplier: f64,
    /// Stable field suffix used by I/O and summary JSON.
    pub field_suffix: &'static str,
}

/// Bollinger Bands parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct BollParams {
    /// Parameter configurations to calculate.
    pub configs: Vec<BollConfig>,
}

impl Default for BollParams {
    fn default() -> Self {
        Self {
            configs: DEFAULT_BOLL_CONFIGS.to_vec(),
        }
    }
}

impl BollParams {
    /// Return true when parameters match the first production canonical set.
    pub fn is_canonical(&self) -> bool {
        self.configs == DEFAULT_BOLL_CONFIGS
    }

    fn validate(&self) -> Result<(), BollError> {
        if self.configs.is_empty() {
            return Err(BollError::InvalidParams(
                "configs must not be empty".to_string(),
            ));
        }
        let mut previous_window = None;
        for config in &self.configs {
            if config.window == 0 {
                return Err(BollError::InvalidParams(
                    "config window must be greater than 0".to_string(),
                ));
            }
            if !config.multiplier.is_finite() || config.multiplier <= 0.0 {
                return Err(BollError::InvalidParams(
                    "config multiplier must be a finite positive number".to_string(),
                ));
            }
            if config.field_suffix.is_empty() {
                return Err(BollError::InvalidParams(
                    "config field_suffix must not be empty".to_string(),
                ));
            }
            if previous_window.is_some_and(|previous| previous >= config.window) {
                return Err(BollError::InvalidParams(
                    "config windows must be strictly increasing".to_string(),
                ));
            }
            previous_window = Some(config.window);
        }
        Ok(())
    }

    /// Return the largest configured rolling window.
    pub fn max_window(&self) -> usize {
        self.configs
            .iter()
            .map(|config| config.window)
            .max()
            .unwrap_or(0)
    }
}

/// One calculated Bollinger band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollBand {
    /// Middle band, `SMA(close, N)`.
    pub mid: Option<f64>,
    /// Upper band, `mid + multiplier * stddev`.
    pub up: Option<f64>,
    /// Lower band, `mid - multiplier * stddev`.
    pub dn: Option<f64>,
}

impl BollBand {
    fn empty() -> Self {
        Self {
            mid: None,
            up: None,
            dn: None,
        }
    }

    fn from_mean_stddev(band: Option<(f64, f64)>, multiplier: f64) -> Self {
        match band {
            Some((mid, stddev)) => Self {
                mid: Some(mid),
                up: Some(mid + multiplier * stddev),
                dn: Some(mid - multiplier * stddev),
            },
            None => Self::empty(),
        }
    }
}

/// Single output row for the canonical Bollinger Bands fields.
#[derive(Debug, Clone, PartialEq)]
pub struct BollOutput {
    /// Trade date copied from input.
    pub trade_date: String,
    /// `BOLL(10, 1.5)` middle band.
    pub boll_mid_10_1p5: Option<f64>,
    /// `BOLL(10, 1.5)` upper band.
    pub boll_up_10_1p5: Option<f64>,
    /// `BOLL(10, 1.5)` lower band.
    pub boll_dn_10_1p5: Option<f64>,
    /// `BOLL(20, 2)` middle band.
    pub boll_mid_20_2: Option<f64>,
    /// `BOLL(20, 2)` upper band.
    pub boll_up_20_2: Option<f64>,
    /// `BOLL(20, 2)` lower band.
    pub boll_dn_20_2: Option<f64>,
    /// `BOLL(50, 2.5)` middle band.
    pub boll_mid_50_2p5: Option<f64>,
    /// `BOLL(50, 2.5)` upper band.
    pub boll_up_50_2p5: Option<f64>,
    /// `BOLL(50, 2.5)` lower band.
    pub boll_dn_50_2p5: Option<f64>,
}

impl BollOutput {
    fn empty(trade_date: impl Into<String>) -> Self {
        Self {
            trade_date: trade_date.into(),
            boll_mid_10_1p5: None,
            boll_up_10_1p5: None,
            boll_dn_10_1p5: None,
            boll_mid_20_2: None,
            boll_up_20_2: None,
            boll_dn_20_2: None,
            boll_mid_50_2p5: None,
            boll_up_50_2p5: None,
            boll_dn_50_2p5: None,
        }
    }

    /// Return true when every business indicator field is null.
    pub fn all_business_indicators_null(&self) -> bool {
        [
            self.boll_mid_10_1p5,
            self.boll_up_10_1p5,
            self.boll_dn_10_1p5,
            self.boll_mid_20_2,
            self.boll_up_20_2,
            self.boll_dn_20_2,
            self.boll_mid_50_2p5,
            self.boll_up_50_2p5,
            self.boll_dn_50_2p5,
        ]
        .iter()
        .all(Option::is_none)
    }

    /// Return the configured band for a `(window, multiplier)` pair.
    pub fn band(&self, window: usize, multiplier: f64) -> Option<BollBand> {
        match (window, multiplier) {
            (10, 1.5) => Some(BollBand {
                mid: self.boll_mid_10_1p5,
                up: self.boll_up_10_1p5,
                dn: self.boll_dn_10_1p5,
            }),
            (20, 2.0) => Some(BollBand {
                mid: self.boll_mid_20_2,
                up: self.boll_up_20_2,
                dn: self.boll_dn_20_2,
            }),
            (50, 2.5) => Some(BollBand {
                mid: self.boll_mid_50_2p5,
                up: self.boll_up_50_2p5,
                dn: self.boll_dn_50_2p5,
            }),
            _ => None,
        }
    }
}

/// Bollinger Bands calculation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BollError {
    /// Invalid parameter values.
    InvalidParams(String),
    /// Input rows are not strictly increasing by trade date.
    NonIncreasingTradeDate {
        /// Previous trade date.
        previous: String,
        /// Current trade date.
        current: String,
    },
    /// Input price is non-finite.
    InvalidPrice,
}

impl fmt::Display for BollError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams(message) => {
                write!(f, "invalid Bollinger Bands parameters: {message}")
            }
            Self::NonIncreasingTradeDate { previous, current } => write!(
                f,
                "input trade_date values must be strictly increasing: previous={previous}, current={current}"
            ),
            Self::InvalidPrice => f.write_str("Bollinger Bands input values must be finite"),
        }
    }
}

impl Error for BollError {}

/// Calculate Bollinger Bands outputs for one ordered security series.
///
/// # Errors
///
/// Returns an error when parameters are invalid, prices are invalid, or trade dates are not
/// strictly increasing.
pub fn calculate_boll_series(
    inputs: &[BollInput],
    params: &BollParams,
) -> Result<Vec<BollOutput>, BollError> {
    params.validate()?;
    validate_sorted(inputs)?;

    let mut operators = params
        .configs
        .iter()
        .map(|config| {
            let operator = RollingMeanStdDev::new(config.window)
                .map_err(|message| BollError::InvalidParams(message.to_string()))?;
            Ok((*config, operator))
        })
        .collect::<Result<Vec<_>, BollError>>()?;
    let mut outputs = Vec::with_capacity(inputs.len());

    for input in inputs {
        let Some(close_price) = input.close_price else {
            outputs.push(BollOutput::empty(input.trade_date.clone()));
            continue;
        };
        if !close_price.is_finite() {
            return Err(BollError::InvalidPrice);
        }

        let mut output = BollOutput::empty(input.trade_date.clone());
        for (config, operator) in &mut operators {
            let band = operator
                .next(Some(close_price))
                .map_err(|_| BollError::InvalidPrice)?;
            output.set_band(
                config.window,
                config.multiplier,
                BollBand::from_mean_stddev(band, config.multiplier),
            );
        }
        outputs.push(output);
    }

    Ok(outputs)
}

impl BollOutput {
    fn set_band(&mut self, window: usize, multiplier: f64, band: BollBand) {
        match (window, multiplier) {
            (10, 1.5) => {
                self.boll_mid_10_1p5 = band.mid;
                self.boll_up_10_1p5 = band.up;
                self.boll_dn_10_1p5 = band.dn;
            }
            (20, 2.0) => {
                self.boll_mid_20_2 = band.mid;
                self.boll_up_20_2 = band.up;
                self.boll_dn_20_2 = band.dn;
            }
            (50, 2.5) => {
                self.boll_mid_50_2p5 = band.mid;
                self.boll_up_50_2p5 = band.up;
                self.boll_dn_50_2p5 = band.dn;
            }
            _ => {}
        }
    }
}

fn validate_sorted(inputs: &[BollInput]) -> Result<(), BollError> {
    let mut previous = None::<&str>;
    for input in inputs {
        if let Some(previous_date) = previous
            && input.trade_date.as_str() <= previous_date
        {
            return Err(BollError::NonIncreasingTradeDate {
                previous: previous_date.to_string(),
                current: input.trade_date.clone(),
            });
        }
        previous = Some(input.trade_date.as_str());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: f64, right: f64) {
        assert!(
            (left - right).abs() < 1e-9,
            "left={left}, right={right}, diff={}",
            (left - right).abs()
        );
    }

    fn inputs(values: &[Option<f64>]) -> Vec<BollInput> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| BollInput::new(format!("2026-01-{:02}", index + 1), *value))
            .collect()
    }

    #[test]
    fn boll_series_calculates_canonical_bands() {
        let values = (1..=50).map(|value| Some(value as f64)).collect::<Vec<_>>();

        let outputs = calculate_boll_series(&inputs(&values), &BollParams::default()).unwrap();

        let day_20 = &outputs[19];
        assert_close(day_20.boll_mid_20_2.unwrap(), 10.5);
        assert_close(day_20.boll_up_20_2.unwrap(), 10.5 + 2.0 * 33.25_f64.sqrt());
        assert_close(day_20.boll_dn_20_2.unwrap(), 10.5 - 2.0 * 33.25_f64.sqrt());
        let day_50 = outputs.last().unwrap();
        assert_close(day_50.boll_mid_50_2p5.unwrap(), 25.5);
        assert_close(
            day_50.boll_up_50_2p5.unwrap(),
            25.5 + 2.5 * 208.25_f64.sqrt(),
        );
        assert_close(
            day_50.boll_dn_50_2p5.unwrap(),
            25.5 - 2.5 * 208.25_f64.sqrt(),
        );
    }

    #[test]
    fn boll_series_outputs_null_row_for_null_close_without_advancing_window() {
        let values = [
            Some(1.0),
            Some(2.0),
            Some(3.0),
            Some(4.0),
            Some(5.0),
            Some(6.0),
            Some(7.0),
            Some(8.0),
            Some(9.0),
            Some(10.0),
            None,
            Some(11.0),
        ];

        let outputs = calculate_boll_series(&inputs(&values), &BollParams::default()).unwrap();

        assert!(outputs[10].all_business_indicators_null());
        assert_close(outputs[11].boll_mid_10_1p5.unwrap(), 6.5);
    }

    #[test]
    fn boll_series_returns_equal_bands_when_stddev_is_zero() {
        let values = (0..20).map(|_| Some(7.0)).collect::<Vec<_>>();

        let outputs = calculate_boll_series(&inputs(&values), &BollParams::default()).unwrap();
        let day_20 = &outputs[19];

        assert_eq!(day_20.boll_mid_20_2, Some(7.0));
        assert_eq!(day_20.boll_up_20_2, Some(7.0));
        assert_eq!(day_20.boll_dn_20_2, Some(7.0));
    }

    #[test]
    fn boll_series_rejects_non_increasing_trade_date() {
        let rows = vec![
            BollInput::new("2026-01-02", Some(1.0)),
            BollInput::new("2026-01-02", Some(2.0)),
        ];

        let error = calculate_boll_series(&rows, &BollParams::default()).unwrap_err();

        assert!(matches!(error, BollError::NonIncreasingTradeDate { .. }));
    }

    #[test]
    fn boll_params_validate_canonical_config() {
        let params = BollParams::default();

        assert!(params.is_canonical());
        assert_eq!(params.max_window(), DEFAULT_BOLL_MAX_WINDOW);
    }
}
