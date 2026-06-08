//! RSV/KDJ 指标计算。

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// Furnace 首版 KDJ 实现使用的默认 RSV 滚动窗口。
pub const DEFAULT_RSV_WINDOW: u16 = 9;

/// Furnace 首版 KDJ 实现使用的默认 K 平滑参数。
pub const DEFAULT_K_SMOOTHING: u16 = 3;

/// Furnace 首版 KDJ 实现使用的默认 D 平滑参数。
pub const DEFAULT_D_SMOOTHING: u16 = 3;

/// 无历史状态时使用的默认初始 K 值。
pub const DEFAULT_INITIAL_K: f64 = 50.0;

/// 无历史状态时使用的默认初始 D 值。
pub const DEFAULT_INITIAL_D: f64 = 50.0;

/// 单只证券在单个交易日的价格输入行。
///
/// 传入 [`calculate_kdj_series`] 的输入必须按 `trade_date` 严格升序排列，
/// 并且全部属于同一只证券的时间序列。
#[derive(Debug, Clone, PartialEq)]
pub struct KdjInput {
    /// 交易日期，使用类似 ISO 日期的可排序字符串表示。
    pub trade_date: String,
    /// 前复权最高价。
    pub high_price: Option<f64>,
    /// 前复权最低价。
    pub low_price: Option<f64>,
    /// 前复权收盘价。
    pub close_price: Option<f64>,
}

impl KdjInput {
    /// 创建一行新的 KDJ 输入数据。
    ///
    /// # 示例
    ///
    /// ```
    /// use furnace_core::KdjInput;
    ///
    /// let input = KdjInput::new("2026-01-01", Some(10.0), Some(8.0), Some(9.0));
    /// assert_eq!(input.trade_date, "2026-01-01");
    /// ```
    pub fn new(
        trade_date: impl Into<String>,
        high_price: Option<f64>,
        low_price: Option<f64>,
        close_price: Option<f64>,
    ) -> Self {
        Self {
            trade_date: trade_date.into(),
            high_price,
            low_price,
            close_price,
        }
    }
}

/// RSV 滚动窗口中保留的有效价格柱。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceBar {
    /// 最高价。
    pub high_price: f64,
    /// 最低价。
    pub low_price: f64,
    /// 收盘价。
    pub close_price: f64,
}

impl PriceBar {
    /// 创建一根已通过校验的价格柱。
    ///
    /// # 错误
    ///
    /// 当任一价格不是有限数，或 `high_price < low_price` 时，
    /// 返回 [`KdjError::InvalidPrice`]。
    pub fn new(high_price: f64, low_price: f64, close_price: f64) -> Result<Self, KdjError> {
        let bar = Self {
            high_price,
            low_price,
            close_price,
        };
        if !bar.is_valid() {
            return Err(KdjError::InvalidPrice);
        }
        Ok(bar)
    }

    fn is_valid(self) -> bool {
        self.high_price.is_finite()
            && self.low_price.is_finite()
            && self.close_price.is_finite()
            && self.high_price >= self.low_price
    }
}

/// KDJ 参数集。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KdjParams {
    /// RSV 滚动窗口长度。
    pub rsv_window: u16,
    /// K 值平滑分母。
    pub k_smoothing: u16,
    /// D 值平滑分母。
    pub d_smoothing: u16,
    /// 仅在不存在有效历史 K/D 状态时使用的初始 K 值。
    pub initial_k: f64,
    /// 仅在不存在有效历史 K/D 状态时使用的初始 D 值。
    pub initial_d: f64,
}

impl Default for KdjParams {
    fn default() -> Self {
        Self {
            rsv_window: DEFAULT_RSV_WINDOW,
            k_smoothing: DEFAULT_K_SMOOTHING,
            d_smoothing: DEFAULT_D_SMOOTHING,
            initial_k: DEFAULT_INITIAL_K,
            initial_d: DEFAULT_INITIAL_D,
        }
    }
}

impl KdjParams {
    /// 判断参数是否为首个生产标准 KDJ 参数集。
    pub fn is_canonical(self) -> bool {
        self.rsv_window == DEFAULT_RSV_WINDOW
            && self.k_smoothing == DEFAULT_K_SMOOTHING
            && self.d_smoothing == DEFAULT_D_SMOOTHING
    }

    fn validate(self) -> Result<(), KdjError> {
        if self.rsv_window == 0 {
            return Err(KdjError::InvalidParams("rsv_window must be greater than 0"));
        }
        if self.k_smoothing == 0 {
            return Err(KdjError::InvalidParams(
                "k_smoothing must be greater than 0",
            ));
        }
        if self.d_smoothing == 0 {
            return Err(KdjError::InvalidParams(
                "d_smoothing must be greater than 0",
            ));
        }
        if !self.initial_k.is_finite() || !self.initial_d.is_finite() {
            return Err(KdjError::InvalidParams("initial K/D values must be finite"));
        }
        Ok(())
    }
}

/// 从最近一次有效 KDJ 输出延续下来的递归 K/D 状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KdjState {
    /// 最近一次有效 K 值。
    pub k_value: f64,
    /// 最近一次有效 D 值。
    pub d_value: f64,
}

impl KdjState {
    /// 创建新的 K/D 状态。
    pub fn new(k_value: f64, d_value: f64) -> Self {
        Self { k_value, d_value }
    }
}

/// 单行输入对应的 KDJ 输出。
#[derive(Debug, Clone, PartialEq)]
pub struct KdjOutput {
    /// 从输入行复制的交易日期。
    pub trade_date: String,
    /// RSV 值。`None` 表示该行尚无完整有效的 RSV 窗口。
    pub rsv: Option<f64>,
    /// K 值。`None` 表示该行无法计算 RSV。
    pub k_value: Option<f64>,
    /// D 值。`None` 表示该行无法计算 RSV。
    pub d_value: Option<f64>,
    /// J 值。`None` 表示该行无法计算 RSV。
    pub j_value: Option<f64>,
}

impl KdjOutput {
    fn empty(trade_date: impl Into<String>) -> Self {
        Self {
            trade_date: trade_date.into(),
            rsv: None,
            k_value: None,
            d_value: None,
            j_value: None,
        }
    }
}

/// KDJ 计算返回的错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KdjError {
    /// 参数值不可用。
    InvalidParams(&'static str),
    /// 输入行没有按交易日期严格升序排列。
    NonIncreasingTradeDate {
        /// 前一行交易日期。
        previous: String,
        /// 当前行交易日期。
        current: String,
    },
    /// 价格柱包含非有限数，或满足 `high_price < low_price`。
    InvalidPrice,
}

impl fmt::Display for KdjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams(message) => write!(f, "invalid KDJ parameters: {message}"),
            Self::NonIncreasingTradeDate { previous, current } => write!(
                f,
                "input trade_date values must be strictly increasing: previous={previous}, current={current}"
            ),
            Self::InvalidPrice => write!(
                f,
                "price bar values must be finite and high_price must be greater than or equal to low_price"
            ),
        }
    }
}

impl Error for KdjError {}

/// 基于完整 RSV 窗口和上一轮状态计算单步 KDJ。
///
/// 适用于已经自行维护 RSV 滚动窗口和上一轮 K/D 状态的增量调用方。
///
/// 当窗口内最高价等于最低价时，按常见行情数据约定将 RSV 置为 `50.0`。
///
/// # 错误
///
/// 当参数无效、价格窗口为空或窗口内包含非法价格时，返回 [`KdjError`]。
///
/// # 示例
///
/// ```
/// use furnace_core::{calculate_kdj_next, KdjParams, KdjState, PriceBar};
///
/// let window = [
///     PriceBar::new(10.0, 8.0, 9.0).unwrap(),
///     PriceBar::new(11.0, 8.0, 10.0).unwrap(),
///     PriceBar::new(12.0, 8.0, 11.0).unwrap(),
/// ];
/// let (rsv, state, j) = calculate_kdj_next(&window, KdjParams { rsv_window: 3, ..KdjParams::default() }, None).unwrap();
/// assert!((rsv - 75.0).abs() < 1e-9);
/// assert!(state.k_value > 50.0);
/// assert!(j > state.d_value);
/// ```
pub fn calculate_kdj_next(
    window: &[PriceBar],
    params: KdjParams,
    previous_state: Option<KdjState>,
) -> Result<(f64, KdjState, f64), KdjError> {
    params.validate()?;
    if window.is_empty() {
        return Err(KdjError::InvalidParams("price window must not be empty"));
    }

    let mut lowest_low = f64::INFINITY;
    let mut highest_high = f64::NEG_INFINITY;
    let mut close_price = None;

    for bar in window {
        if !bar.is_valid() {
            return Err(KdjError::InvalidPrice);
        }
        lowest_low = lowest_low.min(bar.low_price);
        highest_high = highest_high.max(bar.high_price);
        close_price = Some(bar.close_price);
    }

    let Some(close_price) = close_price else {
        return Err(KdjError::InvalidParams("price window must not be empty"));
    };
    let denominator = highest_high - lowest_low;
    let rsv = if denominator == 0.0 {
        50.0
    } else {
        (close_price - lowest_low) / denominator * 100.0
    };

    let previous_state = previous_state.unwrap_or(KdjState {
        k_value: params.initial_k,
        d_value: params.initial_d,
    });
    let k_smoothing = f64::from(params.k_smoothing);
    let d_smoothing = f64::from(params.d_smoothing);
    let k_value =
        ((k_smoothing - 1.0) / k_smoothing) * previous_state.k_value + (1.0 / k_smoothing) * rsv;
    let d_value = ((d_smoothing - 1.0) / d_smoothing) * previous_state.d_value
        + (1.0 / d_smoothing) * k_value;
    let j_value = 3.0 * k_value - 2.0 * d_value;

    Ok((rsv, KdjState { k_value, d_value }, j_value))
}

/// 为单只证券的有序时间序列计算 KDJ 输出。
///
/// 无效或不完整的输入行会输出 `None` 指标字段，并且不会推进递归 K/D 状态。
/// 日级增量运行应传入上一轮 `previous_state`；当其为 `None` 时，
/// `initial_k=50` 和 `initial_d=50` 只用于第一次有效 RSV。
///
/// # 错误
///
/// 当参数无效，或输入的 `trade_date` 未严格递增时，返回 [`KdjError`]。
pub fn calculate_kdj_series(
    inputs: &[KdjInput],
    params: KdjParams,
    previous_state: Option<KdjState>,
) -> Result<Vec<KdjOutput>, KdjError> {
    params.validate()?;
    validate_sorted(inputs)?;

    let mut window = VecDeque::with_capacity(usize::from(params.rsv_window));
    let mut state = previous_state;
    let mut outputs = Vec::with_capacity(inputs.len());

    for input in inputs {
        let Some(bar) = price_bar(input) else {
            outputs.push(KdjOutput::empty(input.trade_date.clone()));
            continue;
        };

        window.push_back(bar);
        while window.len() > usize::from(params.rsv_window) {
            window.pop_front();
        }

        if window.len() < usize::from(params.rsv_window) {
            outputs.push(KdjOutput::empty(input.trade_date.clone()));
            continue;
        }

        let contiguous_window = window.iter().copied().collect::<Vec<_>>();
        let (rsv, next_state, j_value) = calculate_kdj_next(&contiguous_window, params, state)?;
        state = Some(next_state);
        outputs.push(KdjOutput {
            trade_date: input.trade_date.clone(),
            rsv: Some(rsv),
            k_value: Some(next_state.k_value),
            d_value: Some(next_state.d_value),
            j_value: Some(j_value),
        });
    }

    Ok(outputs)
}

fn validate_sorted(inputs: &[KdjInput]) -> Result<(), KdjError> {
    for pair in inputs.windows(2) {
        let previous = &pair[0].trade_date;
        let current = &pair[1].trade_date;
        if current <= previous {
            return Err(KdjError::NonIncreasingTradeDate {
                previous: previous.clone(),
                current: current.clone(),
            });
        }
    }
    Ok(())
}

fn price_bar(input: &KdjInput) -> Option<PriceBar> {
    let high_price = input.high_price?;
    let low_price = input.low_price?;
    let close_price = input.close_price?;
    PriceBar::new(high_price, low_price, close_price).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(day: u8, high: Option<f64>, low: Option<f64>, close: Option<f64>) -> KdjInput {
        KdjInput::new(format!("2026-01-{day:02}"), high, low, close)
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn calculate_kdj_series_outputs_none_until_window_is_complete() {
        let params = KdjParams {
            rsv_window: 3,
            ..KdjParams::default()
        };
        let inputs = vec![
            input(1, Some(10.0), Some(8.0), Some(9.0)),
            input(2, Some(11.0), Some(8.5), Some(10.0)),
        ];

        let outputs = calculate_kdj_series(&inputs, params, None).unwrap();

        assert!(outputs.iter().all(|output| output.rsv.is_none()));
    }

    #[test]
    fn calculate_kdj_series_sets_rsv_to_50_when_denominator_is_zero() {
        let params = KdjParams {
            rsv_window: 2,
            ..KdjParams::default()
        };
        let inputs = vec![
            input(1, Some(10.0), Some(10.0), Some(10.0)),
            input(2, Some(10.0), Some(10.0), Some(10.0)),
        ];

        let outputs = calculate_kdj_series(&inputs, params, None).unwrap();

        assert_close(outputs[1].rsv.unwrap(), 50.0);
    }

    #[test]
    fn calculate_kdj_series_uses_previous_state_for_first_valid_row() {
        let params = KdjParams {
            rsv_window: 1,
            ..KdjParams::default()
        };
        let inputs = vec![input(1, Some(10.0), Some(0.0), Some(10.0))];

        let outputs =
            calculate_kdj_series(&inputs, params, Some(KdjState::new(20.0, 30.0))).unwrap();

        assert_close(outputs[0].k_value.unwrap(), 46.666666666666664);
    }

    #[test]
    fn calculate_kdj_series_missing_price_does_not_advance_state() {
        let params = KdjParams {
            rsv_window: 1,
            ..KdjParams::default()
        };
        let inputs = vec![
            input(1, Some(10.0), Some(0.0), Some(10.0)),
            input(2, None, Some(0.0), Some(10.0)),
            input(3, Some(10.0), Some(0.0), Some(10.0)),
        ];

        let outputs = calculate_kdj_series(&inputs, params, None).unwrap();

        assert_eq!(outputs[1].k_value, None);
        assert_close(outputs[2].k_value.unwrap(), 77.77777777777777);
    }

    #[test]
    fn calculate_kdj_series_rejects_non_increasing_trade_dates() {
        let inputs = vec![
            input(2, Some(10.0), Some(8.0), Some(9.0)),
            input(2, Some(11.0), Some(8.5), Some(10.0)),
        ];

        let error = calculate_kdj_series(&inputs, KdjParams::default(), None).unwrap_err();

        assert!(matches!(error, KdjError::NonIncreasingTradeDate { .. }));
    }

    #[test]
    fn calculate_kdj_series_treats_high_less_than_low_as_invalid_row() {
        let params = KdjParams {
            rsv_window: 1,
            ..KdjParams::default()
        };
        let inputs = vec![input(1, Some(8.0), Some(10.0), Some(9.0))];

        let outputs = calculate_kdj_series(&inputs, params, None).unwrap();

        assert_eq!(outputs[0].rsv, None);
    }

    #[test]
    fn kdj_params_identifies_canonical_parameter_set() {
        assert!(KdjParams::default().is_canonical());
    }
}
