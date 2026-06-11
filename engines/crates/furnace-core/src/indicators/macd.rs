//! MACD daily indicator calculation.

use std::error::Error;
use std::fmt;

use crate::operators::{EmaState, SmaSeededEma};

/// Canonical fast EMA window for the first production MACD version.
pub const DEFAULT_MACD_FAST_WINDOW: usize = 12;
/// Canonical slow EMA window for the first production MACD version.
pub const DEFAULT_MACD_SLOW_WINDOW: usize = 26;
/// Canonical signal DEA EMA window for the first production MACD version.
pub const DEFAULT_MACD_SIGNAL_WINDOW: usize = 9;

/// Single security MACD input row.
#[derive(Debug, Clone, PartialEq)]
pub struct MacdInput {
    /// Trade date represented as an ISO-like sortable string.
    pub trade_date: String,
    /// Forward-adjusted close price.
    pub close_price: Option<f64>,
}

impl MacdInput {
    /// Create a MACD input row.
    pub fn new(trade_date: impl Into<String>, close_price: Option<f64>) -> Self {
        Self {
            trade_date: trade_date.into(),
            close_price,
        }
    }
}

/// MACD indicator parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacdParams {
    /// Fast EMA window.
    pub fast_window: usize,
    /// Slow EMA window.
    pub slow_window: usize,
    /// Signal DEA EMA window.
    pub signal_window: usize,
}

impl Default for MacdParams {
    fn default() -> Self {
        Self {
            fast_window: DEFAULT_MACD_FAST_WINDOW,
            slow_window: DEFAULT_MACD_SLOW_WINDOW,
            signal_window: DEFAULT_MACD_SIGNAL_WINDOW,
        }
    }
}

impl MacdParams {
    /// Return true when parameters match the first production canonical set.
    pub fn is_canonical(&self) -> bool {
        self.fast_window == DEFAULT_MACD_FAST_WINDOW
            && self.slow_window == DEFAULT_MACD_SLOW_WINDOW
            && self.signal_window == DEFAULT_MACD_SIGNAL_WINDOW
    }

    fn validate(&self) -> Result<(), MacdError> {
        if self.fast_window == 0 || self.slow_window == 0 || self.signal_window == 0 {
            return Err(MacdError::InvalidParams(
                "MACD windows must be greater than 0".to_string(),
            ));
        }
        if self.fast_window >= self.slow_window {
            return Err(MacdError::InvalidParams(
                "fast_window must be less than slow_window".to_string(),
            ));
        }
        Ok(())
    }
}

/// Complete MACD state that can continue canonical recursive calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdState {
    /// Last valid EMA(fast) state.
    pub ema_fast_state: f64,
    /// Last valid EMA(slow) state.
    pub ema_slow_state: f64,
    /// Last valid DEA(signal) state.
    pub macd_dea_state: f64,
}

impl MacdState {
    /// Create a complete MACD continuation state.
    ///
    /// # Errors
    ///
    /// Returns an error when any state value is not finite.
    pub fn new(
        ema_fast_state: f64,
        ema_slow_state: f64,
        macd_dea_state: f64,
    ) -> Result<Self, MacdError> {
        if !ema_fast_state.is_finite() || !ema_slow_state.is_finite() || !macd_dea_state.is_finite()
        {
            return Err(MacdError::InvalidPrice);
        }
        Ok(Self {
            ema_fast_state,
            ema_slow_state,
            macd_dea_state,
        })
    }
}

/// Dated MACD previous state.
#[derive(Debug, Clone, PartialEq)]
pub struct MacdPreviousState {
    /// State trade date.
    pub trade_date: String,
    /// Complete MACD state after that trade date.
    pub state: MacdState,
}

impl MacdPreviousState {
    /// Create a dated MACD previous state.
    pub fn new(trade_date: impl Into<String>, state: MacdState) -> Self {
        Self {
            trade_date: trade_date.into(),
            state,
        }
    }
}

/// Single MACD output row.
#[derive(Debug, Clone, PartialEq)]
pub struct MacdOutput {
    /// Trade date copied from input.
    pub trade_date: String,
    /// EMA(12) continuation state.
    pub ema_fast_state_12: Option<f64>,
    /// EMA(26) continuation state.
    pub ema_slow_state_26: Option<f64>,
    /// DIF, `EMA(12) - EMA(26)`.
    pub macd_dif: Option<f64>,
    /// DEA, `EMA(DIF, 9)`.
    pub macd_dea: Option<f64>,
    /// DEA continuation state.
    pub macd_dea_state: Option<f64>,
    /// Standard histogram, `DIF - DEA`.
    pub macd_histogram: Option<f64>,
}

impl MacdOutput {
    /// Return true when every business MACD field is null.
    pub fn all_business_indicators_null(&self) -> bool {
        [self.macd_dif, self.macd_dea, self.macd_histogram]
            .iter()
            .all(Option::is_none)
    }
}

/// MACD output values without the copied trade date.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdOutputValues {
    /// EMA(12) continuation state.
    pub ema_fast_state_12: Option<f64>,
    /// EMA(26) continuation state.
    pub ema_slow_state_26: Option<f64>,
    /// DIF, `EMA(12) - EMA(26)`.
    pub macd_dif: Option<f64>,
    /// DEA, `EMA(DIF, 9)`.
    pub macd_dea: Option<f64>,
    /// DEA continuation state.
    pub macd_dea_state: Option<f64>,
    /// Standard histogram, `DIF - DEA`.
    pub macd_histogram: Option<f64>,
}

impl MacdOutputValues {
    fn empty() -> Self {
        Self {
            ema_fast_state_12: None,
            ema_slow_state_26: None,
            macd_dif: None,
            macd_dea: None,
            macd_dea_state: None,
            macd_histogram: None,
        }
    }

    /// Return true when every business MACD field is null.
    pub fn all_business_indicators_null(&self) -> bool {
        [self.macd_dif, self.macd_dea, self.macd_histogram]
            .iter()
            .all(Option::is_none)
    }
}

/// MACD calculation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacdError {
    /// Invalid parameter values.
    InvalidParams(String),
    /// Input rows are not strictly increasing by trade date.
    NonIncreasingTradeDate {
        /// Previous trade date.
        previous: String,
        /// Current trade date.
        current: String,
    },
    /// Input price or state is non-finite.
    InvalidPrice,
}

impl fmt::Display for MacdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams(message) => write!(f, "invalid MACD parameters: {message}"),
            Self::NonIncreasingTradeDate { previous, current } => write!(
                f,
                "input trade_date values must be strictly increasing: previous={previous}, current={current}"
            ),
            Self::InvalidPrice => f.write_str("MACD input values and states must be finite"),
        }
    }
}

impl Error for MacdError {}

/// Calculate MACD outputs for one ordered security series.
///
/// # Errors
///
/// Returns an error when parameters are invalid, input prices are invalid, or
/// input `trade_date` values are not strictly increasing.
pub fn calculate_macd(
    inputs: &[MacdInput],
    params: &MacdParams,
    previous_state: Option<MacdState>,
) -> Result<Vec<MacdOutput>, MacdError> {
    calculate_macd_internal(inputs, params, previous_state, None)
}

/// Calculate MACD outputs for one ordered security series.
///
/// # Errors
///
/// Returns an error when parameters are invalid, input prices are invalid, or
/// input `trade_date` values are not strictly increasing.
pub fn calculate_macd_series(
    inputs: &[MacdInput],
    params: &MacdParams,
    previous_state: Option<MacdState>,
) -> Result<Vec<MacdOutput>, MacdError> {
    calculate_macd(inputs, params, previous_state)
}

/// Calculate MACD outputs, skipping rows up to the dated previous state.
///
/// # Errors
///
/// Returns an error when parameters are invalid, input prices are invalid, or
/// input `trade_date` values are not strictly increasing.
pub fn calculate_macd_series_from_previous_state(
    inputs: &[MacdInput],
    params: &MacdParams,
    previous_state: Option<MacdPreviousState>,
) -> Result<Vec<MacdOutput>, MacdError> {
    let mut outputs = Vec::with_capacity(inputs.len());
    visit_macd_series_from_previous_state(inputs, params, previous_state, |trade_date, values| {
        outputs.push(MacdOutput {
            trade_date: trade_date.to_string(),
            ema_fast_state_12: values.ema_fast_state_12,
            ema_slow_state_26: values.ema_slow_state_26,
            macd_dif: values.macd_dif,
            macd_dea: values.macd_dea,
            macd_dea_state: values.macd_dea_state,
            macd_histogram: values.macd_histogram,
        });
    })?;
    Ok(outputs)
}

/// Visit MACD outputs for one ordered security series without allocating an output vector.
///
/// # Errors
///
/// Returns an error when parameters are invalid, input prices are invalid, or
/// input `trade_date` values are not strictly increasing.
pub fn visit_macd_series_from_previous_state(
    inputs: &[MacdInput],
    params: &MacdParams,
    previous_state: Option<MacdPreviousState>,
    visitor: impl FnMut(&str, MacdOutputValues),
) -> Result<(), MacdError> {
    let state = previous_state.as_ref().map(|previous| previous.state);
    let state_date = previous_state
        .as_ref()
        .map(|previous| previous.trade_date.as_str());
    visit_macd_internal(inputs, params, state, state_date, visitor)
}

fn calculate_macd_internal(
    inputs: &[MacdInput],
    params: &MacdParams,
    previous_state: Option<MacdState>,
    previous_state_date: Option<&str>,
) -> Result<Vec<MacdOutput>, MacdError> {
    let mut outputs = Vec::with_capacity(inputs.len());
    visit_macd_internal(
        inputs,
        params,
        previous_state,
        previous_state_date,
        |trade_date, values| {
            outputs.push(MacdOutput {
                trade_date: trade_date.to_string(),
                ema_fast_state_12: values.ema_fast_state_12,
                ema_slow_state_26: values.ema_slow_state_26,
                macd_dif: values.macd_dif,
                macd_dea: values.macd_dea,
                macd_dea_state: values.macd_dea_state,
                macd_histogram: values.macd_histogram,
            });
        },
    )?;
    Ok(outputs)
}

fn visit_macd_internal(
    inputs: &[MacdInput],
    params: &MacdParams,
    previous_state: Option<MacdState>,
    previous_state_date: Option<&str>,
    mut visitor: impl FnMut(&str, MacdOutputValues),
) -> Result<(), MacdError> {
    params.validate()?;
    validate_sorted(inputs)?;

    let mut fast = SmaSeededEma::new(
        params.fast_window,
        previous_state.map(|state| EmaState {
            value: state.ema_fast_state,
        }),
    )
    .map_err(|message| MacdError::InvalidParams(message.to_string()))?;
    let mut slow = SmaSeededEma::new(
        params.slow_window,
        previous_state.map(|state| EmaState {
            value: state.ema_slow_state,
        }),
    )
    .map_err(|message| MacdError::InvalidParams(message.to_string()))?;
    let mut signal = SmaSeededEma::new(
        params.signal_window,
        previous_state.map(|state| EmaState {
            value: state.macd_dea_state,
        }),
    )
    .map_err(|message| MacdError::InvalidParams(message.to_string()))?;
    for input in inputs {
        if previous_state_date.is_some_and(|state_date| input.trade_date.as_str() <= state_date) {
            if input.close_price.is_some_and(|close| !close.is_finite()) {
                return Err(MacdError::InvalidPrice);
            }
            visitor(&input.trade_date, MacdOutputValues::empty());
            continue;
        }

        let Some(close_price) = input.close_price else {
            visitor(&input.trade_date, MacdOutputValues::empty());
            continue;
        };
        if !close_price.is_finite() {
            return Err(MacdError::InvalidPrice);
        }

        let ema_fast = fast
            .next(Some(close_price))
            .map_err(|_| MacdError::InvalidPrice)?;
        let ema_slow = slow
            .next(Some(close_price))
            .map_err(|_| MacdError::InvalidPrice)?;
        let macd_dif = ema_fast.zip(ema_slow).map(|(fast, slow)| fast - slow);
        let macd_dea = signal.next(macd_dif).map_err(|_| MacdError::InvalidPrice)?;
        visitor(
            &input.trade_date,
            MacdOutputValues {
                ema_fast_state_12: ema_fast,
                ema_slow_state_26: ema_slow,
                macd_dif,
                macd_dea,
                macd_dea_state: macd_dea,
                macd_histogram: macd_dif.zip(macd_dea).map(|(dif, dea)| dif - dea),
            },
        );
    }

    Ok(())
}

fn validate_sorted(inputs: &[MacdInput]) -> Result<(), MacdError> {
    let mut previous = None::<&str>;
    for input in inputs {
        if let Some(previous_date) = previous
            && input.trade_date.as_str() <= previous_date
        {
            return Err(MacdError::NonIncreasingTradeDate {
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

    fn inputs(values: &[Option<f64>]) -> Vec<MacdInput> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| MacdInput::new(format!("2026-01-{:02}", index + 1), *value))
            .collect()
    }

    #[test]
    fn macd_params_default_to_canonical_windows() {
        let params = MacdParams::default();

        assert_eq!(params.fast_window, 12);
        assert_eq!(params.slow_window, 26);
        assert_eq!(params.signal_window, 9);
        assert!(params.is_canonical());
    }

    #[test]
    fn macd_should_start_ema_from_sma_seed() {
        let values = (1..=26).map(|value| Some(value as f64)).collect::<Vec<_>>();

        let outputs = calculate_macd(&inputs(&values), &MacdParams::default(), None).unwrap();

        assert!(outputs[10].ema_fast_state_12.is_none());
        assert_close(outputs[11].ema_fast_state_12.unwrap(), 6.5);
        assert!(outputs[24].ema_slow_state_26.is_none());
        assert_close(outputs[25].ema_slow_state_26.unwrap(), 13.5);
    }

    #[test]
    fn macd_should_emit_first_dif_when_fast_and_slow_ema_available() {
        let values = (1..=26).map(|value| Some(value as f64)).collect::<Vec<_>>();

        let outputs = calculate_macd(&inputs(&values), &MacdParams::default(), None).unwrap();

        assert!(outputs[24].macd_dif.is_none());
        assert_close(outputs[25].macd_dif.unwrap(), 7.0);
    }

    #[test]
    fn macd_should_emit_first_dea_after_nine_valid_dif_values() {
        let values = (1..=34).map(|value| Some(value as f64)).collect::<Vec<_>>();

        let outputs = calculate_macd(&inputs(&values), &MacdParams::default(), None).unwrap();

        assert!(outputs[32].macd_dea.is_none());
        assert_close(outputs[33].macd_dea.unwrap(), 7.0);
        assert_close(outputs[33].macd_histogram.unwrap(), 0.0);
    }

    #[test]
    fn macd_should_not_advance_state_when_close_is_null() {
        let values = (1..=34)
            .map(|value| {
                if value == 20 {
                    None
                } else {
                    Some(value as f64)
                }
            })
            .collect::<Vec<_>>();

        let outputs = calculate_macd(&inputs(&values), &MacdParams::default(), None).unwrap();

        assert!(outputs[19].ema_fast_state_12.is_none());
        assert!(outputs[19].ema_slow_state_26.is_none());
        assert!(outputs[19].macd_dif.is_none());
        assert!(outputs[33].macd_dea.is_none());
    }

    #[test]
    fn macd_should_continue_from_previous_state_consistently() {
        let values = (1..=40).map(|value| Some(value as f64)).collect::<Vec<_>>();
        let all_outputs = calculate_macd(&inputs(&values), &MacdParams::default(), None).unwrap();
        let previous = &all_outputs[33];
        let previous_state = MacdPreviousState::new(
            previous.trade_date.clone(),
            MacdState::new(
                previous.ema_fast_state_12.unwrap(),
                previous.ema_slow_state_26.unwrap(),
                previous.macd_dea_state.unwrap(),
            )
            .unwrap(),
        );

        let resumed = calculate_macd_series_from_previous_state(
            &inputs(&values),
            &MacdParams::default(),
            Some(previous_state),
        )
        .unwrap();

        for index in 34..40 {
            assert_close(
                resumed[index].ema_fast_state_12.unwrap(),
                all_outputs[index].ema_fast_state_12.unwrap(),
            );
            assert_close(
                resumed[index].ema_slow_state_26.unwrap(),
                all_outputs[index].ema_slow_state_26.unwrap(),
            );
            assert_close(
                resumed[index].macd_dea.unwrap(),
                all_outputs[index].macd_dea.unwrap(),
            );
            assert_close(
                resumed[index].macd_histogram.unwrap(),
                all_outputs[index].macd_histogram.unwrap(),
            );
        }
    }

    #[test]
    fn macd_should_reject_non_finite_close() {
        let mut values = (1..=34).map(|value| Some(value as f64)).collect::<Vec<_>>();
        values[3] = Some(f64::INFINITY);

        let error = calculate_macd(&inputs(&values), &MacdParams::default(), None).unwrap_err();

        assert_eq!(error, MacdError::InvalidPrice);
    }
}
