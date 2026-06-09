//! Price action and previous-low/second-low daily structure indicators.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// Canonical previous-low/second-low structure window.
pub const DEFAULT_N_STRUCTURE_WINDOW: usize = 20;

/// Single security price-pattern input row.
#[derive(Debug, Clone, PartialEq)]
pub struct PricePatternInput {
    /// Trade date represented as an ISO-like sortable string.
    pub trade_date: String,
    /// Forward-adjusted high price used by structure detection.
    pub high_price: Option<f64>,
    /// Forward-adjusted low price used by structure detection.
    pub low_price: Option<f64>,
    /// Unadjusted close price used by close streaks.
    pub close_price: Option<f64>,
    /// Unadjusted previous close price used by close streaks.
    pub prev_close_price: Option<f64>,
}

impl PricePatternInput {
    /// Create a price-pattern input row.
    pub fn new(
        trade_date: impl Into<String>,
        high_price: Option<f64>,
        low_price: Option<f64>,
        close_price: Option<f64>,
        prev_close_price: Option<f64>,
    ) -> Self {
        Self {
            trade_date: trade_date.into(),
            high_price,
            low_price,
            close_price,
            prev_close_price,
        }
    }
}

/// Price-pattern parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricePatternParams {
    /// Number of recent valid high/low bars used by structure fields.
    pub n_structure_window: usize,
}

impl Default for PricePatternParams {
    fn default() -> Self {
        Self {
            n_structure_window: DEFAULT_N_STRUCTURE_WINDOW,
        }
    }
}

impl PricePatternParams {
    /// Return true when parameters match the production canonical set.
    pub fn is_canonical(&self) -> bool {
        self.n_structure_window == DEFAULT_N_STRUCTURE_WINDOW
    }

    fn validate(&self) -> Result<(), PricePatternError> {
        if self.n_structure_window == 0 {
            return Err(PricePatternError::InvalidParams(
                "n_structure_window must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Carry-forward price-pattern state.
#[derive(Debug, Clone, PartialEq)]
pub struct PricePatternState {
    /// Current consecutive up days.
    pub up_streak_days: u16,
    /// Current consecutive down days.
    pub down_streak_days: u16,
    /// Last known close direction, if any.
    pub last_direction: Option<i8>,
    /// Recent valid high/low bars for structure detection.
    pub structure_window: Vec<StructurePriceBar>,
}

impl PricePatternState {
    /// Create an empty state.
    pub fn empty() -> Self {
        Self {
            up_streak_days: 0,
            down_streak_days: 0,
            last_direction: None,
            structure_window: Vec::new(),
        }
    }
}

impl Default for PricePatternState {
    fn default() -> Self {
        Self::empty()
    }
}

/// Price-pattern state with its trade date.
#[derive(Debug, Clone, PartialEq)]
pub struct PricePatternPreviousState {
    /// State trade date.
    pub trade_date: String,
    /// State available after that trade date.
    pub state: PricePatternState,
}

impl PricePatternPreviousState {
    /// Create dated previous state.
    pub fn new(trade_date: impl Into<String>, state: PricePatternState) -> Self {
        Self {
            trade_date: trade_date.into(),
            state,
        }
    }
}

/// Valid high/low bar retained in the structure window.
#[derive(Debug, Clone, PartialEq)]
pub struct StructurePriceBar {
    /// Trade date copied from input.
    pub trade_date: String,
    /// Forward-adjusted high price.
    pub high_price: f64,
    /// Forward-adjusted low price.
    pub low_price: f64,
}

/// Single price-pattern output row.
#[derive(Debug, Clone, PartialEq)]
pub struct PricePatternOutput {
    /// Trade date copied from input.
    pub trade_date: String,
    /// Close direction: 1 up, -1 down, 0 flat.
    pub close_direction: Option<i8>,
    /// Consecutive up days.
    pub close_up_streak_days: Option<u16>,
    /// Consecutive down days.
    pub close_down_streak_days: Option<u16>,
    /// Recent valid high/low bars in the structure window.
    pub n_structure_20_valid_bars: u16,
    /// First highest high date in the window.
    pub n_structure_20_high_date: Option<String>,
    /// Highest high price in the window.
    pub n_structure_20_high_price: Option<f64>,
    /// First lowest low date on the left side including the high bar.
    pub n_structure_20_low_date: Option<String>,
    /// Lowest low price on the left side including the high bar.
    pub n_structure_20_low_price: Option<f64>,
    /// First lowest low date on the right side after the high bar.
    pub n_structure_20_second_low_date: Option<String>,
    /// Lowest low price on the right side after the high bar.
    pub n_structure_20_second_low_price: Option<f64>,
    /// second_low / low.
    pub n_structure_20_second_low_ratio: Option<f64>,
    /// Whether second_low is strictly greater than low.
    pub n_structure_20_is_valid: bool,
}

impl PricePatternOutput {
    fn new(trade_date: impl Into<String>, structure: StructureOutput) -> Self {
        Self {
            trade_date: trade_date.into(),
            close_direction: None,
            close_up_streak_days: None,
            close_down_streak_days: None,
            n_structure_20_valid_bars: structure.valid_bars,
            n_structure_20_high_date: structure.high_date,
            n_structure_20_high_price: structure.high_price,
            n_structure_20_low_date: structure.low_date,
            n_structure_20_low_price: structure.low_price,
            n_structure_20_second_low_date: structure.second_low_date,
            n_structure_20_second_low_price: structure.second_low_price,
            n_structure_20_second_low_ratio: structure.second_low_ratio,
            n_structure_20_is_valid: structure.is_valid,
        }
    }
}

/// Price-pattern calculation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PricePatternError {
    /// Invalid parameter values.
    InvalidParams(String),
    /// Input rows are not strictly increasing by trade date.
    NonIncreasingTradeDate {
        /// Previous trade date.
        previous: String,
        /// Current trade date.
        current: String,
    },
    /// Input price is non-finite or previous state contains an invalid structure bar.
    InvalidPrice,
    /// Close streak counter exceeded UInt16 capacity.
    StreakOverflow,
}

impl fmt::Display for PricePatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams(message) => {
                write!(f, "invalid price-pattern parameters: {message}")
            }
            Self::NonIncreasingTradeDate { previous, current } => write!(
                f,
                "input trade_date values must be strictly increasing: previous={previous}, current={current}"
            ),
            Self::InvalidPrice => f.write_str("price-pattern input values must be finite"),
            Self::StreakOverflow => f.write_str("price-pattern close streak exceeded UInt16"),
        }
    }
}

impl Error for PricePatternError {}

/// Calculate price-pattern outputs for one ordered security series.
///
/// # Errors
///
/// Returns an error when parameters are invalid, prices are invalid, streaks overflow, or trade
/// dates are not strictly increasing.
pub fn calculate_price_pattern_series(
    inputs: &[PricePatternInput],
    params: &PricePatternParams,
    previous_state: Option<PricePatternPreviousState>,
) -> Result<Vec<PricePatternOutput>, PricePatternError> {
    params.validate()?;
    validate_sorted(inputs)?;

    let mut state = previous_state
        .as_ref()
        .map(|previous| previous.state.clone())
        .unwrap_or_default();
    validate_previous_state(&state)?;
    let previous_state_date = previous_state
        .as_ref()
        .map(|previous| previous.trade_date.as_str());
    let mut structure_window = VecDeque::from(std::mem::take(&mut state.structure_window));
    truncate_structure_window(&mut structure_window, params.n_structure_window);

    let mut outputs = Vec::with_capacity(inputs.len());
    for input in inputs {
        validate_input_prices(input)?;

        if previous_state_date.is_some_and(|state_date| input.trade_date.as_str() <= state_date) {
            outputs.push(PricePatternOutput::new(
                input.trade_date.clone(),
                structure_output(&structure_window),
            ));
            continue;
        }

        if let Some(bar) = structure_bar(input)? {
            structure_window.push_back(bar);
            truncate_structure_window(&mut structure_window, params.n_structure_window);
        }
        let structure = structure_output(&structure_window);
        let mut output = PricePatternOutput::new(input.trade_date.clone(), structure);

        match close_direction(input) {
            Some(1) => {
                state.up_streak_days = state
                    .up_streak_days
                    .checked_add(1)
                    .ok_or(PricePatternError::StreakOverflow)?;
                state.down_streak_days = 0;
                state.last_direction = Some(1);
                output.close_direction = Some(1);
                output.close_up_streak_days = Some(state.up_streak_days);
                output.close_down_streak_days = Some(0);
            }
            Some(-1) => {
                state.down_streak_days = state
                    .down_streak_days
                    .checked_add(1)
                    .ok_or(PricePatternError::StreakOverflow)?;
                state.up_streak_days = 0;
                state.last_direction = Some(-1);
                output.close_direction = Some(-1);
                output.close_up_streak_days = Some(0);
                output.close_down_streak_days = Some(state.down_streak_days);
            }
            Some(0) => {
                state.up_streak_days = 0;
                state.down_streak_days = 0;
                state.last_direction = Some(0);
                output.close_direction = Some(0);
                output.close_up_streak_days = Some(0);
                output.close_down_streak_days = Some(0);
            }
            Some(_) => unreachable!("close_direction only returns -1, 0, or 1"),
            None => {
                state.up_streak_days = 0;
                state.down_streak_days = 0;
                state.last_direction = None;
            }
        }

        outputs.push(output);
    }

    Ok(outputs)
}

fn validate_sorted(inputs: &[PricePatternInput]) -> Result<(), PricePatternError> {
    let mut previous = None::<&str>;
    for input in inputs {
        if let Some(previous_date) = previous
            && input.trade_date.as_str() <= previous_date
        {
            return Err(PricePatternError::NonIncreasingTradeDate {
                previous: previous_date.to_string(),
                current: input.trade_date.clone(),
            });
        }
        previous = Some(input.trade_date.as_str());
    }
    Ok(())
}

fn validate_previous_state(state: &PricePatternState) -> Result<(), PricePatternError> {
    if state
        .last_direction
        .is_some_and(|direction| !matches!(direction, -1..=1))
    {
        return Err(PricePatternError::InvalidPrice);
    }
    let mut previous = None::<&str>;
    for bar in &state.structure_window {
        validate_structure_values(bar.high_price, bar.low_price)?;
        if let Some(previous_date) = previous
            && bar.trade_date.as_str() <= previous_date
        {
            return Err(PricePatternError::NonIncreasingTradeDate {
                previous: previous_date.to_string(),
                current: bar.trade_date.clone(),
            });
        }
        previous = Some(bar.trade_date.as_str());
    }
    Ok(())
}

fn validate_input_prices(input: &PricePatternInput) -> Result<(), PricePatternError> {
    for value in [
        input.high_price,
        input.low_price,
        input.close_price,
        input.prev_close_price,
    ]
    .into_iter()
    .flatten()
    {
        if !value.is_finite() {
            return Err(PricePatternError::InvalidPrice);
        }
    }
    Ok(())
}

fn validate_structure_values(high: f64, low: f64) -> Result<(), PricePatternError> {
    if !high.is_finite() || !low.is_finite() || low <= 0.0 || high < low {
        return Err(PricePatternError::InvalidPrice);
    }
    Ok(())
}

fn structure_bar(
    input: &PricePatternInput,
) -> Result<Option<StructurePriceBar>, PricePatternError> {
    let (Some(high_price), Some(low_price)) = (input.high_price, input.low_price) else {
        return Ok(None);
    };
    if low_price <= 0.0 || high_price < low_price {
        return Ok(None);
    }
    Ok(Some(StructurePriceBar {
        trade_date: input.trade_date.clone(),
        high_price,
        low_price,
    }))
}

fn close_direction(input: &PricePatternInput) -> Option<i8> {
    let close = input.close_price?;
    let previous = input.prev_close_price?;
    if close > previous {
        Some(1)
    } else if close < previous {
        Some(-1)
    } else {
        Some(0)
    }
}

fn truncate_structure_window(window: &mut VecDeque<StructurePriceBar>, max_len: usize) {
    while window.len() > max_len {
        window.pop_front();
    }
}

#[derive(Debug, Clone, PartialEq)]
struct StructureOutput {
    valid_bars: u16,
    high_date: Option<String>,
    high_price: Option<f64>,
    low_date: Option<String>,
    low_price: Option<f64>,
    second_low_date: Option<String>,
    second_low_price: Option<f64>,
    second_low_ratio: Option<f64>,
    is_valid: bool,
}

fn structure_output(window: &VecDeque<StructurePriceBar>) -> StructureOutput {
    let valid_bars = u16::try_from(window.len()).unwrap_or(u16::MAX);
    let Some(high_index) = first_high_index(window) else {
        return StructureOutput {
            valid_bars,
            high_date: None,
            high_price: None,
            low_date: None,
            low_price: None,
            second_low_date: None,
            second_low_price: None,
            second_low_ratio: None,
            is_valid: false,
        };
    };
    let high = &window[high_index];
    let low = first_low(&window.range(..=high_index).collect::<Vec<_>>());
    let second_low = if high_index + 1 < window.len() {
        first_low(&window.range(high_index + 1..).collect::<Vec<_>>())
    } else {
        None
    };
    let second_low_ratio = match (low, second_low) {
        (Some(low), Some(second_low)) if low.low_price > 0.0 => {
            Some(second_low.low_price / low.low_price)
        }
        _ => None,
    };

    StructureOutput {
        valid_bars,
        high_date: Some(high.trade_date.clone()),
        high_price: Some(high.high_price),
        low_date: low.map(|bar| bar.trade_date.clone()),
        low_price: low.map(|bar| bar.low_price),
        second_low_date: second_low.map(|bar| bar.trade_date.clone()),
        second_low_price: second_low.map(|bar| bar.low_price),
        second_low_ratio,
        is_valid: second_low_ratio.is_some_and(|ratio| ratio > 1.0),
    }
}

fn first_high_index(window: &VecDeque<StructurePriceBar>) -> Option<usize> {
    let mut best = None::<(usize, f64)>;
    for (index, bar) in window.iter().enumerate() {
        match best {
            Some((_, price)) if price >= bar.high_price => {}
            _ => best = Some((index, bar.high_price)),
        }
    }
    best.map(|(index, _)| index)
}

fn first_low<'a>(bars: &[&'a StructurePriceBar]) -> Option<&'a StructurePriceBar> {
    let mut best = None::<&StructurePriceBar>;
    for bar in bars {
        match best {
            Some(current) if current.low_price <= bar.low_price => {}
            _ => best = Some(*bar),
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        day: u8,
        high: Option<f64>,
        low: Option<f64>,
        close: Option<f64>,
        prev_close: Option<f64>,
    ) -> PricePatternInput {
        PricePatternInput::new(format!("2026-01-{day:02}"), high, low, close, prev_close)
    }

    fn assert_close(left: f64, right: f64) {
        assert!(
            (left - right).abs() < 1e-9,
            "left={left}, right={right}, diff={}",
            (left - right).abs()
        );
    }

    #[test]
    fn close_streaks_count_up_down_and_flat_days() {
        let rows = vec![
            row(1, None, None, Some(11.0), Some(10.0)),
            row(2, None, None, Some(12.0), Some(11.0)),
            row(3, None, None, Some(12.0), Some(12.0)),
            row(4, None, None, Some(10.0), Some(12.0)),
            row(5, None, None, Some(9.0), Some(10.0)),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();

        assert_eq!(outputs[0].close_direction, Some(1));
        assert_eq!(outputs[0].close_up_streak_days, Some(1));
        assert_eq!(outputs[1].close_up_streak_days, Some(2));
        assert_eq!(outputs[2].close_direction, Some(0));
        assert_eq!(outputs[2].close_up_streak_days, Some(0));
        assert_eq!(outputs[2].close_down_streak_days, Some(0));
        assert_eq!(outputs[3].close_direction, Some(-1));
        assert_eq!(outputs[3].close_down_streak_days, Some(1));
        assert_eq!(outputs[4].close_down_streak_days, Some(2));
    }

    #[test]
    fn null_close_inputs_break_streak_and_output_null_streak_fields() {
        let rows = vec![
            row(1, None, None, Some(11.0), Some(10.0)),
            row(2, None, None, None, Some(11.0)),
            row(3, None, None, Some(12.0), Some(11.0)),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();

        assert_eq!(outputs[1].close_direction, None);
        assert_eq!(outputs[1].close_up_streak_days, None);
        assert_eq!(outputs[1].close_down_streak_days, None);
        assert_eq!(outputs[2].close_up_streak_days, Some(1));
    }

    #[test]
    fn missing_structure_prices_do_not_advance_window() {
        let rows = vec![
            row(1, None, None, Some(1.0), Some(1.0)),
            row(2, Some(10.0), Some(8.0), Some(1.0), Some(1.0)),
            row(3, None, Some(7.0), Some(1.0), Some(1.0)),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();

        assert_eq!(outputs[0].n_structure_20_valid_bars, 0);
        assert!(outputs[0].n_structure_20_high_price.is_none());
        assert_eq!(outputs[2].n_structure_20_valid_bars, 1);
        assert_eq!(outputs[2].n_structure_20_high_price, Some(10.0));
    }

    #[test]
    fn structure_uses_first_high_left_low_and_right_second_low() {
        let rows = vec![
            row(1, Some(10.0), Some(7.0), None, None),
            row(2, Some(15.0), Some(8.0), None, None),
            row(3, Some(15.0), Some(6.0), None, None),
            row(4, Some(12.0), Some(9.0), None, None),
            row(5, Some(11.0), Some(9.5), None, None),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert_eq!(last.n_structure_20_high_date.as_deref(), Some("2026-01-02"));
        assert_eq!(last.n_structure_20_low_date.as_deref(), Some("2026-01-01"));
        assert_eq!(
            last.n_structure_20_second_low_date.as_deref(),
            Some("2026-01-03")
        );
        assert_close(last.n_structure_20_second_low_ratio.unwrap(), 6.0 / 7.0);
        assert!(!last.n_structure_20_is_valid);
    }

    #[test]
    fn structure_is_valid_when_second_low_ratio_is_above_one() {
        let rows = vec![
            row(1, Some(10.0), Some(5.0), None, None),
            row(2, Some(15.0), Some(7.0), None, None),
            row(3, Some(12.0), Some(8.0), None, None),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert_close(last.n_structure_20_second_low_ratio.unwrap(), 8.0 / 5.0);
        assert!(last.n_structure_20_is_valid);
    }

    #[test]
    fn structure_keeps_only_recent_twenty_valid_bars() {
        let rows = (1..=21)
            .map(|day| row(day, Some(day as f64), Some(day as f64), None, None))
            .collect::<Vec<_>>();

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert_eq!(last.n_structure_20_valid_bars, 20);
        assert_eq!(last.n_structure_20_low_date.as_deref(), Some("2026-01-02"));
    }

    #[test]
    fn high_at_last_bar_has_no_second_low_or_ratio() {
        let rows = vec![
            row(1, Some(10.0), Some(6.0), None, None),
            row(2, Some(20.0), Some(7.0), None, None),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert!(last.n_structure_20_second_low_price.is_none());
        assert!(last.n_structure_20_second_low_ratio.is_none());
        assert!(!last.n_structure_20_is_valid);
    }

    #[test]
    fn previous_state_continuation_matches_full_history_for_streaks_and_structure() {
        let rows = vec![
            row(1, Some(10.0), Some(5.0), Some(11.0), Some(10.0)),
            row(2, Some(15.0), Some(7.0), Some(12.0), Some(11.0)),
            row(3, Some(12.0), Some(8.0), Some(13.0), Some(12.0)),
        ];
        let full =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let state = PricePatternState {
            up_streak_days: 2,
            down_streak_days: 0,
            last_direction: Some(1),
            structure_window: vec![
                StructurePriceBar {
                    trade_date: "2026-01-01".to_string(),
                    high_price: 10.0,
                    low_price: 5.0,
                },
                StructurePriceBar {
                    trade_date: "2026-01-02".to_string(),
                    high_price: 15.0,
                    low_price: 7.0,
                },
            ],
        };
        let continued = calculate_price_pattern_series(
            &rows[2..],
            &PricePatternParams::default(),
            Some(PricePatternPreviousState::new("2026-01-02", state)),
        )
        .unwrap();

        assert_eq!(continued[0], full[2]);
    }

    #[test]
    fn rejects_non_increasing_trade_date_and_non_finite_prices() {
        let duplicate = vec![
            row(1, Some(1.0), Some(1.0), None, None),
            row(1, Some(1.0), Some(1.0), None, None),
        ];
        let error =
            calculate_price_pattern_series(&duplicate, &PricePatternParams::default(), None)
                .unwrap_err();
        assert!(matches!(
            error,
            PricePatternError::NonIncreasingTradeDate { .. }
        ));

        let invalid = vec![row(1, Some(f64::INFINITY), Some(1.0), None, None)];
        let error = calculate_price_pattern_series(&invalid, &PricePatternParams::default(), None)
            .unwrap_err();
        assert_eq!(error, PricePatternError::InvalidPrice);
    }

    #[test]
    fn unusable_structure_bars_do_not_advance_window() {
        let rows = vec![
            row(1, Some(10.0), Some(5.0), None, None),
            row(2, Some(0.0), Some(0.0), None, None),
            row(3, Some(4.0), Some(6.0), None, None),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();

        assert_eq!(outputs[0].n_structure_20_valid_bars, 1);
        assert_eq!(outputs[1].n_structure_20_valid_bars, 1);
        assert_eq!(outputs[2].n_structure_20_valid_bars, 1);
        assert_eq!(
            outputs[2].n_structure_20_high_date.as_deref(),
            Some("2026-01-01")
        );
    }
}
