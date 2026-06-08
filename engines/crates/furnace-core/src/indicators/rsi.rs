//! RSI 日线指标计算。

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// 第一版生产 RSI 窗口集合。
pub const DEFAULT_RSI_WINDOWS: [usize; 6] = [6, 12, 14, 24, 25, 50];

/// 单只证券在单个交易日的 RSI 输入行。
#[derive(Debug, Clone, PartialEq)]
pub struct RsiInput {
    /// 交易日期，使用类似 ISO 日期的可排序字符串表示。
    pub trade_date: String,
    /// 前复权收盘价。
    pub close_price: Option<f64>,
}

impl RsiInput {
    /// 创建 RSI 输入行。
    pub fn new(trade_date: impl Into<String>, close_price: Option<f64>) -> Self {
        Self {
            trade_date: trade_date.into(),
            close_price,
        }
    }
}

/// RSI 指标参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsiParams {
    /// RSI 窗口集合。
    pub windows: Vec<usize>,
}

impl Default for RsiParams {
    fn default() -> Self {
        Self {
            windows: DEFAULT_RSI_WINDOWS.to_vec(),
        }
    }
}

impl RsiParams {
    /// 判断参数是否为首版生产 canonical 参数。
    pub fn is_canonical(&self) -> bool {
        self.windows == DEFAULT_RSI_WINDOWS
    }

    fn validate(&self) -> Result<(), RsiError> {
        if self.windows.is_empty() {
            return Err(RsiError::InvalidParams(
                "windows must not be empty".to_string(),
            ));
        }
        let mut previous = None;
        for window in &self.windows {
            if *window == 0 {
                return Err(RsiError::InvalidParams(
                    "windows must be greater than 0".to_string(),
                ));
            }
            if previous.is_some_and(|previous| previous >= *window) {
                return Err(RsiError::InvalidParams(
                    "windows must be strictly increasing".to_string(),
                ));
            }
            previous = Some(*window);
        }
        Ok(())
    }
}

/// 单个 RSI 窗口可延续的 Wilder 平滑状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RsiWindowState {
    /// 最近一次有效平均涨幅。
    pub avg_gain: f64,
    /// 最近一次有效平均跌幅。
    pub avg_loss: f64,
}

impl RsiWindowState {
    /// 创建 RSI 窗口状态。
    ///
    /// # 错误
    ///
    /// 当状态值不是有限数或为负时返回错误。
    pub fn new(avg_gain: f64, avg_loss: f64) -> Result<Self, RsiError> {
        if !avg_gain.is_finite() || !avg_loss.is_finite() || avg_gain < 0.0 || avg_loss < 0.0 {
            return Err(RsiError::InvalidPrice);
        }
        Ok(Self { avg_gain, avg_loss })
    }
}

/// 可用于延续 RSI 递推的完整 canonical 状态。
#[derive(Debug, Clone, PartialEq)]
pub struct RsiState {
    /// previous state 日期对应的有效 close。
    pub previous_close: f64,
    /// 6 日 RSI 状态。
    pub rsi_6: RsiWindowState,
    /// 12 日 RSI 状态。
    pub rsi_12: RsiWindowState,
    /// 14 日 RSI 状态。
    pub rsi_14: RsiWindowState,
    /// 24 日 RSI 状态。
    pub rsi_24: RsiWindowState,
    /// 25 日 RSI 状态。
    pub rsi_25: RsiWindowState,
    /// 50 日 RSI 状态。
    pub rsi_50: RsiWindowState,
}

impl RsiState {
    /// 创建完整 RSI 状态。
    ///
    /// # 错误
    ///
    /// 当 `previous_close` 不是有限数时返回错误。
    pub fn new(
        previous_close: f64,
        rsi_6: RsiWindowState,
        rsi_12: RsiWindowState,
        rsi_14: RsiWindowState,
        rsi_24: RsiWindowState,
        rsi_25: RsiWindowState,
        rsi_50: RsiWindowState,
    ) -> Result<Self, RsiError> {
        if !previous_close.is_finite() {
            return Err(RsiError::InvalidPrice);
        }
        Ok(Self {
            previous_close,
            rsi_6,
            rsi_12,
            rsi_14,
            rsi_24,
            rsi_25,
            rsi_50,
        })
    }

    fn window_state(&self, window: usize) -> Option<RsiWindowState> {
        match window {
            6 => Some(self.rsi_6),
            12 => Some(self.rsi_12),
            14 => Some(self.rsi_14),
            24 => Some(self.rsi_24),
            25 => Some(self.rsi_25),
            50 => Some(self.rsi_50),
            _ => None,
        }
    }
}

/// 带日期的 RSI previous state。
#[derive(Debug, Clone, PartialEq)]
pub struct RsiPreviousState {
    /// 状态对应交易日期。
    pub trade_date: String,
    /// 该交易日收盘后可延续的 RSI 状态。
    pub state: RsiState,
}

impl RsiPreviousState {
    /// 创建带日期的 RSI 状态。
    pub fn new(trade_date: impl Into<String>, state: RsiState) -> Self {
        Self {
            trade_date: trade_date.into(),
            state,
        }
    }
}

/// 单行 RSI 输出。
#[derive(Debug, Clone, PartialEq)]
pub struct RsiOutput {
    /// 从输入行复制的交易日期。
    pub trade_date: String,
    /// RSI(6)。
    pub rsi_6: Option<f64>,
    /// RSI(12)。
    pub rsi_12: Option<f64>,
    /// RSI(14)。
    pub rsi_14: Option<f64>,
    /// RSI(24)。
    pub rsi_24: Option<f64>,
    /// RSI(25)。
    pub rsi_25: Option<f64>,
    /// RSI(50)。
    pub rsi_50: Option<f64>,
    /// RSI(6) 平均涨幅状态。
    pub avg_gain_6_state: Option<f64>,
    /// RSI(6) 平均跌幅状态。
    pub avg_loss_6_state: Option<f64>,
    /// RSI(12) 平均涨幅状态。
    pub avg_gain_12_state: Option<f64>,
    /// RSI(12) 平均跌幅状态。
    pub avg_loss_12_state: Option<f64>,
    /// RSI(14) 平均涨幅状态。
    pub avg_gain_14_state: Option<f64>,
    /// RSI(14) 平均跌幅状态。
    pub avg_loss_14_state: Option<f64>,
    /// RSI(24) 平均涨幅状态。
    pub avg_gain_24_state: Option<f64>,
    /// RSI(24) 平均跌幅状态。
    pub avg_loss_24_state: Option<f64>,
    /// RSI(25) 平均涨幅状态。
    pub avg_gain_25_state: Option<f64>,
    /// RSI(25) 平均跌幅状态。
    pub avg_loss_25_state: Option<f64>,
    /// RSI(50) 平均涨幅状态。
    pub avg_gain_50_state: Option<f64>,
    /// RSI(50) 平均跌幅状态。
    pub avg_loss_50_state: Option<f64>,
}

impl RsiOutput {
    fn empty(trade_date: impl Into<String>) -> Self {
        Self {
            trade_date: trade_date.into(),
            rsi_6: None,
            rsi_12: None,
            rsi_14: None,
            rsi_24: None,
            rsi_25: None,
            rsi_50: None,
            avg_gain_6_state: None,
            avg_loss_6_state: None,
            avg_gain_12_state: None,
            avg_loss_12_state: None,
            avg_gain_14_state: None,
            avg_loss_14_state: None,
            avg_gain_24_state: None,
            avg_loss_24_state: None,
            avg_gain_25_state: None,
            avg_loss_25_state: None,
            avg_gain_50_state: None,
            avg_loss_50_state: None,
        }
    }

    /// 所有业务 RSI 字段是否均为空。
    pub fn all_business_indicators_null(&self) -> bool {
        [
            self.rsi_6,
            self.rsi_12,
            self.rsi_14,
            self.rsi_24,
            self.rsi_25,
            self.rsi_50,
        ]
        .iter()
        .all(Option::is_none)
    }

    /// 读取指定窗口的 RSI 值。
    pub fn rsi(&self, window: usize) -> Option<f64> {
        match window {
            6 => self.rsi_6,
            12 => self.rsi_12,
            14 => self.rsi_14,
            24 => self.rsi_24,
            25 => self.rsi_25,
            50 => self.rsi_50,
            _ => None,
        }
    }

    fn set_window(&mut self, window: usize, state: Option<RsiWindowState>) {
        let rsi = state.map(rsi_from_state);
        let avg_gain = state.map(|state| state.avg_gain);
        let avg_loss = state.map(|state| state.avg_loss);
        match window {
            6 => {
                self.rsi_6 = rsi;
                self.avg_gain_6_state = avg_gain;
                self.avg_loss_6_state = avg_loss;
            }
            12 => {
                self.rsi_12 = rsi;
                self.avg_gain_12_state = avg_gain;
                self.avg_loss_12_state = avg_loss;
            }
            14 => {
                self.rsi_14 = rsi;
                self.avg_gain_14_state = avg_gain;
                self.avg_loss_14_state = avg_loss;
            }
            24 => {
                self.rsi_24 = rsi;
                self.avg_gain_24_state = avg_gain;
                self.avg_loss_24_state = avg_loss;
            }
            25 => {
                self.rsi_25 = rsi;
                self.avg_gain_25_state = avg_gain;
                self.avg_loss_25_state = avg_loss;
            }
            50 => {
                self.rsi_50 = rsi;
                self.avg_gain_50_state = avg_gain;
                self.avg_loss_50_state = avg_loss;
            }
            _ => {}
        }
    }
}

/// RSI 计算错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RsiError {
    /// 参数值不可用。
    InvalidParams(String),
    /// 输入行没有按交易日期严格升序排列。
    NonIncreasingTradeDate {
        /// 前一行交易日期。
        previous: String,
        /// 当前行交易日期。
        current: String,
    },
    /// 输入价格或状态值不是有限数，或状态值为负。
    InvalidPrice,
}

impl fmt::Display for RsiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams(message) => write!(f, "invalid RSI parameters: {message}"),
            Self::NonIncreasingTradeDate { previous, current } => write!(
                f,
                "input trade_date values must be strictly increasing: previous={previous}, current={current}"
            ),
            Self::InvalidPrice => {
                f.write_str("RSI input values and states must be finite and non-negative")
            }
        }
    }
}

impl Error for RsiError {}

/// 为单只证券的有序时间序列计算 RSI 输出。
///
/// # 错误
///
/// 当参数无效、价格无效或输入 `trade_date` 未严格递增时返回错误。
pub fn calculate_rsi_series(
    inputs: &[RsiInput],
    params: &RsiParams,
    previous_state: Option<RsiState>,
) -> Result<Vec<RsiOutput>, RsiError> {
    calculate_rsi_series_internal(inputs, params, previous_state, None)
}

/// 为单只证券的有序时间序列计算 RSI 输出，并从指定日期后的行延续状态。
///
/// # 错误
///
/// 当参数无效、价格无效或输入 `trade_date` 未严格递增时返回错误。
pub fn calculate_rsi_series_from_previous_state(
    inputs: &[RsiInput],
    params: &RsiParams,
    previous_state: Option<RsiPreviousState>,
) -> Result<Vec<RsiOutput>, RsiError> {
    let state = previous_state
        .as_ref()
        .map(|previous| previous.state.clone());
    let state_date = previous_state
        .as_ref()
        .map(|previous| previous.trade_date.as_str());
    calculate_rsi_series_internal(inputs, params, state, state_date)
}

fn calculate_rsi_series_internal(
    inputs: &[RsiInput],
    params: &RsiParams,
    previous_state: Option<RsiState>,
    previous_state_date: Option<&str>,
) -> Result<Vec<RsiOutput>, RsiError> {
    params.validate()?;
    validate_sorted(inputs)?;

    let mut calculators = params
        .windows
        .iter()
        .map(|window| {
            let state = previous_state
                .as_ref()
                .and_then(|state| state.window_state(*window));
            WilderAverage::new(*window, state)
                .map(|average| (*window, average))
                .map_err(|message| RsiError::InvalidParams(message.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut previous_close = previous_state.as_ref().map(|state| state.previous_close);
    let mut outputs = Vec::with_capacity(inputs.len());

    for input in inputs {
        if previous_state_date.is_some_and(|state_date| input.trade_date.as_str() <= state_date) {
            if input.close_price.is_some_and(|close| !close.is_finite()) {
                return Err(RsiError::InvalidPrice);
            }
            outputs.push(RsiOutput::empty(input.trade_date.clone()));
            continue;
        }

        let Some(close_price) = input.close_price else {
            outputs.push(RsiOutput::empty(input.trade_date.clone()));
            continue;
        };
        if !close_price.is_finite() {
            return Err(RsiError::InvalidPrice);
        }

        let Some(previous) = previous_close else {
            previous_close = Some(close_price);
            outputs.push(RsiOutput::empty(input.trade_date.clone()));
            continue;
        };

        let change = close_price - previous;
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);
        let mut output = RsiOutput::empty(input.trade_date.clone());
        for (window, calculator) in &mut calculators {
            let state = calculator
                .next(gain, loss)
                .map_err(|_| RsiError::InvalidPrice)?;
            output.set_window(*window, state);
        }
        previous_close = Some(close_price);
        outputs.push(output);
    }

    Ok(outputs)
}

#[derive(Debug, Clone, PartialEq)]
struct WilderAverage {
    window: usize,
    state: Option<RsiWindowState>,
    seed_gains: VecDeque<f64>,
    seed_losses: VecDeque<f64>,
    seed_gain_sum: f64,
    seed_loss_sum: f64,
}

impl WilderAverage {
    fn new(window: usize, previous_state: Option<RsiWindowState>) -> Result<Self, &'static str> {
        if window == 0 {
            return Err("RSI window must be greater than 0");
        }
        Ok(Self {
            window,
            state: previous_state,
            seed_gains: VecDeque::with_capacity(window),
            seed_losses: VecDeque::with_capacity(window),
            seed_gain_sum: 0.0,
            seed_loss_sum: 0.0,
        })
    }

    fn next(&mut self, gain: f64, loss: f64) -> Result<Option<RsiWindowState>, &'static str> {
        if !gain.is_finite() || !loss.is_finite() || gain < 0.0 || loss < 0.0 {
            return Err("RSI gain/loss input must be finite and non-negative");
        }
        if let Some(previous) = self.state {
            let window = self.window as f64;
            let state = RsiWindowState {
                avg_gain: (previous.avg_gain * (window - 1.0) + gain) / window,
                avg_loss: (previous.avg_loss * (window - 1.0) + loss) / window,
            };
            self.state = Some(state);
            return Ok(Some(state));
        }

        self.seed_gains.push_back(gain);
        self.seed_losses.push_back(loss);
        self.seed_gain_sum += gain;
        self.seed_loss_sum += loss;
        if self.seed_gains.len() < self.window {
            return Ok(None);
        }

        let state = RsiWindowState {
            avg_gain: self.seed_gain_sum / self.window as f64,
            avg_loss: self.seed_loss_sum / self.window as f64,
        };
        self.seed_gains.clear();
        self.seed_losses.clear();
        self.seed_gain_sum = 0.0;
        self.seed_loss_sum = 0.0;
        self.state = Some(state);
        Ok(Some(state))
    }
}

fn rsi_from_state(state: RsiWindowState) -> f64 {
    if state.avg_gain == 0.0 && state.avg_loss == 0.0 {
        50.0
    } else if state.avg_loss == 0.0 {
        100.0
    } else if state.avg_gain == 0.0 {
        0.0
    } else {
        let relative_strength = state.avg_gain / state.avg_loss;
        100.0 - 100.0 / (1.0 + relative_strength)
    }
}

fn validate_sorted(inputs: &[RsiInput]) -> Result<(), RsiError> {
    let mut previous = None::<&str>;
    for input in inputs {
        if let Some(previous_date) = previous
            && input.trade_date.as_str() <= previous_date
        {
            return Err(RsiError::NonIncreasingTradeDate {
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

    fn inputs(values: &[Option<f64>]) -> Vec<RsiInput> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| RsiInput::new(format!("2026-01-{:02}", index + 1), *value))
            .collect()
    }

    fn state_with(value: RsiWindowState, previous_close: f64) -> RsiState {
        RsiState::new(previous_close, value, value, value, value, value, value).unwrap()
    }

    #[test]
    fn canonical_params_use_requested_windows() {
        let params = RsiParams::default();

        assert_eq!(params.windows, vec![6, 12, 14, 24, 25, 50]);
        assert!(params.is_canonical());
    }

    #[test]
    fn rsi_series_outputs_empty_for_empty_input() {
        let outputs = calculate_rsi_series(&[], &RsiParams::default(), None).unwrap();

        assert!(outputs.is_empty());
    }

    #[test]
    fn rsi_6_first_non_null_occurs_on_7th_valid_close() {
        let values = (1..=7).map(|value| Some(value as f64)).collect::<Vec<_>>();
        let outputs = calculate_rsi_series(&inputs(&values), &RsiParams::default(), None).unwrap();

        assert!(outputs[5].rsi_6.is_none());
        assert_eq!(outputs[6].rsi_6, Some(100.0));
        assert_eq!(outputs[6].avg_loss_6_state, Some(0.0));
    }

    #[test]
    fn rsi_50_first_non_null_occurs_on_51st_valid_close() {
        let values = (1..=51).map(|value| Some(value as f64)).collect::<Vec<_>>();
        let outputs = calculate_rsi_series(&inputs(&values), &RsiParams::default(), None).unwrap();

        assert!(outputs[49].rsi_50.is_none());
        assert_eq!(outputs[50].rsi_50, Some(100.0));
    }

    #[test]
    fn rsi_handles_flat_and_down_series_boundaries() {
        let flat = vec![Some(10.0); 7];
        let flat_outputs =
            calculate_rsi_series(&inputs(&flat), &RsiParams::default(), None).unwrap();
        assert_eq!(flat_outputs[6].rsi_6, Some(50.0));

        let down = (0..=6)
            .map(|index| Some(10.0 - index as f64))
            .collect::<Vec<_>>();
        let down_outputs =
            calculate_rsi_series(&inputs(&down), &RsiParams::default(), None).unwrap();
        assert_eq!(down_outputs[6].rsi_6, Some(0.0));
    }

    #[test]
    fn rsi_uses_wilder_smoothing_after_seed() {
        let values = [1.0, 2.0, 3.0, 2.0, 4.0, 5.0, 6.0, 5.0]
            .iter()
            .map(|value| Some(*value))
            .collect::<Vec<_>>();
        let outputs = calculate_rsi_series(&inputs(&values), &RsiParams::default(), None).unwrap();

        assert_close(outputs[6].avg_gain_6_state.unwrap(), 1.0);
        assert_close(outputs[6].avg_loss_6_state.unwrap(), 1.0 / 6.0);
        assert_close(outputs[6].rsi_6.unwrap(), 100.0 - 100.0 / (1.0 + 6.0));
        assert_close(outputs[7].avg_gain_6_state.unwrap(), 5.0 / 6.0);
        assert_close(
            outputs[7].avg_loss_6_state.unwrap(),
            ((1.0 / 6.0) * 5.0 + 1.0) / 6.0,
        );
    }

    #[test]
    fn null_close_does_not_advance_state_or_previous_close() {
        let values = [
            Some(10.0),
            Some(11.0),
            Some(12.0),
            Some(13.0),
            Some(14.0),
            Some(15.0),
            None,
            Some(16.0),
        ];
        let outputs = calculate_rsi_series(&inputs(&values), &RsiParams::default(), None).unwrap();

        assert!(outputs[6].all_business_indicators_null());
        assert_eq!(outputs[7].rsi_6, Some(100.0));
        assert_eq!(outputs[7].avg_gain_6_state, Some(1.0));
        assert_eq!(outputs[7].avg_loss_6_state, Some(0.0));
    }

    #[test]
    fn previous_state_continuation_matches_full_history() {
        let values = [10.0, 11.0, 12.0, 11.0, 13.0, 14.0, 13.0, 15.0, 16.0, 17.0]
            .iter()
            .map(|value| Some(*value))
            .collect::<Vec<_>>();
        let full = calculate_rsi_series(&inputs(&values), &RsiParams::default(), None).unwrap();
        let anchor = full[6].clone();
        let state = RsiState::new(
            13.0,
            RsiWindowState::new(
                anchor.avg_gain_6_state.unwrap(),
                anchor.avg_loss_6_state.unwrap(),
            )
            .unwrap(),
            RsiWindowState::new(0.0, 0.0).unwrap(),
            RsiWindowState::new(0.0, 0.0).unwrap(),
            RsiWindowState::new(0.0, 0.0).unwrap(),
            RsiWindowState::new(0.0, 0.0).unwrap(),
            RsiWindowState::new(0.0, 0.0).unwrap(),
        )
        .unwrap();
        let previous = RsiPreviousState::new("2026-01-07", state);
        let continued = calculate_rsi_series_from_previous_state(
            &inputs(&values),
            &RsiParams::default(),
            Some(previous),
        )
        .unwrap();

        assert_eq!(continued[6].rsi_6, None);
        assert_eq!(continued[7].rsi_6, full[7].rsi_6);
        assert_eq!(continued[9].rsi_6, full[9].rsi_6);
    }

    #[test]
    fn previous_state_anchor_row_is_not_reconsumed_as_zero_change() {
        let window_state = RsiWindowState::new(1.0, 0.0).unwrap();
        let previous = RsiPreviousState::new("2026-01-01", state_with(window_state, 10.0));
        let rows = vec![
            RsiInput::new("2026-01-01", Some(10.0)),
            RsiInput::new("2026-01-02", Some(11.0)),
        ];

        let outputs =
            calculate_rsi_series_from_previous_state(&rows, &RsiParams::default(), Some(previous))
                .unwrap();

        assert!(outputs[0].all_business_indicators_null());
        assert_eq!(outputs[1].rsi_6, Some(100.0));
        assert_eq!(outputs[1].avg_gain_6_state, Some(1.0));
    }

    #[test]
    fn zero_gain_or_loss_states_are_valid_previous_state() {
        let window_state = RsiWindowState::new(0.0, 1.0).unwrap();
        let previous = RsiPreviousState::new("2026-01-01", state_with(window_state, 10.0));
        let rows = vec![
            RsiInput::new("2026-01-01", Some(10.0)),
            RsiInput::new("2026-01-02", Some(9.0)),
        ];

        let outputs =
            calculate_rsi_series_from_previous_state(&rows, &RsiParams::default(), Some(previous))
                .unwrap();

        assert_eq!(outputs[1].rsi_6, Some(0.0));
        assert_eq!(outputs[1].avg_gain_6_state, Some(0.0));
    }

    #[test]
    fn rsi_series_rejects_non_increasing_trade_date() {
        let rows = vec![
            RsiInput::new("2026-01-01", Some(1.0)),
            RsiInput::new("2026-01-01", Some(2.0)),
        ];

        let error = calculate_rsi_series(&rows, &RsiParams::default(), None).unwrap_err();

        assert!(matches!(error, RsiError::NonIncreasingTradeDate { .. }));
    }

    #[test]
    fn rsi_series_rejects_non_finite_close() {
        let rows = vec![RsiInput::new("2026-01-01", Some(f64::INFINITY))];

        let error = calculate_rsi_series(&rows, &RsiParams::default(), None).unwrap_err();

        assert_eq!(error, RsiError::InvalidPrice);
    }
}
