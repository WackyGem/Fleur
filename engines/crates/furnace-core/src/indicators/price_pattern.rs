//! Price action and L1/H1/L2 rebound daily structure indicators.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// Canonical L1/H1/L2 rebound structure window.
pub const DEFAULT_N_STRUCTURE_WINDOW: usize = 20;
const MIN_N_STRUCTURE_FIRST_LEG_PCT: f64 = 0.08;
const MIN_N_STRUCTURE_HIGHER_LOW_RATIO: f64 = 1.01;
const MIN_N_STRUCTURE_PULLBACK_DEPTH: f64 = 0.25;
const MAX_N_STRUCTURE_PULLBACK_DEPTH: f64 = 0.75;
const MIN_N_STRUCTURE_REBOUND_RATIO: f64 = 1.03;

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
    /// Whether a complete L1 -> H1 -> L2 -> current rebound N structure is present.
    pub n_structure_20_is_valid: bool,
    /// N structure stage: none, higher_low, rebound, or breakout.
    pub n_structure_20_stage: String,
    /// L2 / L1.
    pub n_structure_20_higher_low_ratio: Option<f64>,
    /// (H1 - L2) / (H1 - L1).
    pub n_structure_20_pullback_depth: Option<f64>,
    /// Current valid high / L2.
    pub n_structure_20_rebound_ratio: Option<f64>,
}

impl PricePatternOutput {
    fn new(trade_date: impl Into<String>, structure: StructureOutput) -> Self {
        Self {
            trade_date: trade_date.into(),
            close_direction: None,
            close_up_streak_days: None,
            close_down_streak_days: None,
            n_structure_20_is_valid: structure.is_valid,
            n_structure_20_stage: structure.stage.to_string(),
            n_structure_20_higher_low_ratio: structure.higher_low_ratio,
            n_structure_20_pullback_depth: structure.pullback_depth,
            n_structure_20_rebound_ratio: structure.rebound_ratio,
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
    is_valid: bool,
    stage: &'static str,
    higher_low_ratio: Option<f64>,
    pullback_depth: Option<f64>,
    rebound_ratio: Option<f64>,
}

fn structure_output(window: &VecDeque<StructurePriceBar>) -> StructureOutput {
    let Some(candidate) = best_n_structure_candidate(window) else {
        return StructureOutput::default();
    };

    StructureOutput {
        is_valid: matches!(
            candidate.stage,
            NStructureStage::Rebound | NStructureStage::Breakout
        ),
        stage: candidate.stage.as_str(),
        higher_low_ratio: Some(candidate.higher_low_ratio),
        pullback_depth: Some(candidate.pullback_depth),
        rebound_ratio: Some(candidate.rebound_ratio),
    }
}

impl Default for StructureOutput {
    fn default() -> Self {
        Self {
            is_valid: false,
            stage: NStructureStage::None.as_str(),
            higher_low_ratio: None,
            pullback_depth: None,
            rebound_ratio: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum NStructureStage {
    None,
    HigherLow,
    Rebound,
    Breakout,
}

impl NStructureStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::HigherLow => "higher_low",
            Self::Rebound => "rebound",
            Self::Breakout => "breakout",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct NStructureCandidate {
    stage: NStructureStage,
    higher_low_ratio: f64,
    pullback_depth: f64,
    rebound_ratio: f64,
    h1_index: usize,
    l2_index: usize,
}

fn best_n_structure_candidate(window: &VecDeque<StructurePriceBar>) -> Option<NStructureCandidate> {
    if window.len() < 4 {
        return None;
    }

    let current_index = window.len() - 1;
    let current = &window[current_index];
    let mut best = None::<NStructureCandidate>;

    for l1_index in 0..current_index {
        let l1 = &window[l1_index];
        for h1_index in l1_index + 1..current_index {
            let h1 = &window[h1_index];
            let first_leg = h1.high_price / l1.low_price - 1.0;
            if first_leg < MIN_N_STRUCTURE_FIRST_LEG_PCT {
                continue;
            }

            let leg_height = h1.high_price - l1.low_price;
            if leg_height <= 0.0 {
                continue;
            }

            for (l2_index, l2) in window
                .iter()
                .enumerate()
                .take(current_index)
                .skip(h1_index + 1)
            {
                let higher_low_ratio = l2.low_price / l1.low_price;
                if higher_low_ratio < MIN_N_STRUCTURE_HIGHER_LOW_RATIO {
                    continue;
                }

                let pullback_depth = (h1.high_price - l2.low_price) / leg_height;
                if !(MIN_N_STRUCTURE_PULLBACK_DEPTH..=MAX_N_STRUCTURE_PULLBACK_DEPTH)
                    .contains(&pullback_depth)
                {
                    continue;
                }

                let rebound_ratio = current.high_price / l2.low_price;
                let stage = if current.high_price >= h1.high_price {
                    NStructureStage::Breakout
                } else if rebound_ratio >= MIN_N_STRUCTURE_REBOUND_RATIO {
                    NStructureStage::Rebound
                } else {
                    NStructureStage::HigherLow
                };
                let candidate = NStructureCandidate {
                    stage,
                    higher_low_ratio,
                    pullback_depth,
                    rebound_ratio,
                    h1_index,
                    l2_index,
                };
                if best.is_none_or(|current_best| is_better_candidate(candidate, current_best)) {
                    best = Some(candidate);
                }
            }
        }
    }

    best
}

fn is_better_candidate(candidate: NStructureCandidate, best: NStructureCandidate) -> bool {
    candidate.stage > best.stage
        || (candidate.stage == best.stage && candidate.l2_index > best.l2_index)
        || (candidate.stage == best.stage
            && candidate.l2_index == best.l2_index
            && candidate.h1_index > best.h1_index)
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

        assert_eq!(outputs[0].n_structure_20_stage, "none");
        assert_eq!(outputs[2].n_structure_20_stage, "none");
        assert!(outputs[2].n_structure_20_higher_low_ratio.is_none());
    }

    #[test]
    fn n_structure_marks_higher_low_before_rebound_as_not_valid() {
        let rows = vec![
            row(1, Some(11.0), Some(10.0), None, None),
            row(2, Some(20.0), Some(18.0), None, None),
            row(3, Some(16.0), Some(13.0), None, None),
            row(4, Some(13.2), Some(12.8), None, None),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert_eq!(last.n_structure_20_stage, "higher_low");
        assert!(!last.n_structure_20_is_valid);
        assert_close(last.n_structure_20_higher_low_ratio.unwrap(), 13.0 / 10.0);
        assert_close(last.n_structure_20_pullback_depth.unwrap(), 0.7);
    }

    #[test]
    fn n_structure_is_valid_when_current_bar_rebounds_from_l2() {
        let rows = vec![
            row(1, Some(11.0), Some(10.0), None, None),
            row(2, Some(20.0), Some(18.0), None, None),
            row(3, Some(16.0), Some(13.0), None, None),
            row(4, Some(14.0), Some(13.2), None, None),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert_eq!(last.n_structure_20_stage, "rebound");
        assert!(last.n_structure_20_is_valid);
        assert_close(last.n_structure_20_rebound_ratio.unwrap(), 14.0 / 13.0);
    }

    #[test]
    fn n_structure_marks_breakout_when_current_bar_clears_h1() {
        let rows = vec![
            row(1, Some(11.0), Some(10.0), None, None),
            row(2, Some(20.0), Some(18.0), None, None),
            row(3, Some(16.0), Some(13.0), None, None),
            row(4, Some(20.1), Some(14.0), None, None),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert_eq!(last.n_structure_20_stage, "breakout");
        assert!(last.n_structure_20_is_valid);
    }

    #[test]
    fn n_structure_ignores_l1_outside_recent_twenty_valid_bars() {
        let mut rows = vec![
            row(1, Some(11.0), Some(10.0), None, None),
            row(2, Some(20.0), Some(18.0), None, None),
            row(3, Some(16.0), Some(13.0), None, None),
        ];
        rows.extend((4..=21).map(|day| row(day, Some(12.0), Some(11.0), None, None)));

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert_eq!(last.n_structure_20_stage, "none");
        assert!(!last.n_structure_20_is_valid);
    }

    #[test]
    fn n_structure_requires_l2_before_current_bar() {
        let rows = vec![
            row(1, Some(10.0), Some(6.0), None, None),
            row(2, Some(20.0), Some(7.0), None, None),
        ];

        let outputs =
            calculate_price_pattern_series(&rows, &PricePatternParams::default(), None).unwrap();
        let last = outputs.last().unwrap();

        assert_eq!(last.n_structure_20_stage, "none");
        assert!(last.n_structure_20_rebound_ratio.is_none());
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

        assert_eq!(outputs[0].n_structure_20_stage, "none");
        assert_eq!(outputs[1].n_structure_20_stage, "none");
        assert_eq!(outputs[2].n_structure_20_stage, "none");
    }
}
