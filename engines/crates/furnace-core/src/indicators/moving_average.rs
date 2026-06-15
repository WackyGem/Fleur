//! Moving Average 日线指标计算。

use std::error::Error;
use std::fmt;

use crate::operators::{RollingSma, SmaSeededEma};

/// 第一版生产价格 MA 窗口集合。
pub const DEFAULT_PRICE_MA_WINDOWS: [usize; 14] =
    [3, 5, 6, 10, 12, 14, 20, 24, 28, 30, 57, 60, 114, 250];

/// 第一版生产均量 MA 窗口集合。
pub const DEFAULT_VOLUME_MA_WINDOWS: [usize; 4] = [5, 10, 20, 60];

/// 第一版生产 EMA 窗口。
pub const DEFAULT_EMA_WINDOW: usize = 10;

/// 单只证券在单个交易日的 MA 输入行。
#[derive(Debug, Clone, PartialEq)]
pub struct MaInput {
    /// 交易日期，使用类似 ISO 日期的可排序字符串表示。
    pub trade_date: String,
    /// 前复权收盘价。
    pub close_price: Option<f64>,
    /// 未复权日行情成交量。0 是有效成交量。
    pub volume: Option<f64>,
}

impl MaInput {
    /// 创建 MA 输入行。
    pub fn new(
        trade_date: impl Into<String>,
        close_price: Option<f64>,
        volume: Option<f64>,
    ) -> Self {
        Self {
            trade_date: trade_date.into(),
            close_price,
            volume,
        }
    }
}

/// EMA 指标递推状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaState {
    /// 最近一次有效 EMA(close, 10)。
    pub price_ema1_10_state: f64,
    /// 最近一次有效 EMA(EMA(close, 10), 10)。
    pub price_ema2_10_state: f64,
}

impl MaState {
    /// 创建可延续的 MA EMA 状态。
    ///
    /// # 错误
    ///
    /// 当任一状态值不是有限数时返回错误。
    pub fn new(price_ema1_10_state: f64, price_ema2_10_state: f64) -> Result<Self, MaError> {
        if !price_ema1_10_state.is_finite() || !price_ema2_10_state.is_finite() {
            return Err(MaError::InvalidPrice);
        }
        Ok(Self {
            price_ema1_10_state,
            price_ema2_10_state,
        })
    }
}

/// 带日期的 EMA 指标递推状态。
///
/// 当调用方为了 MA 窗口读取了早于状态日期的 lookback 行时，
/// 该结构用于确保 EMA 只从 `trade_date` 之后的输入继续递推。
#[derive(Debug, Clone, PartialEq)]
pub struct MaPreviousState {
    /// 状态对应的交易日期。
    pub trade_date: String,
    /// 该交易日收盘后可延续的 EMA 状态。
    pub state: MaState,
}

impl MaPreviousState {
    /// 创建带日期的 MA EMA 状态。
    pub fn new(trade_date: impl Into<String>, state: MaState) -> Self {
        Self {
            trade_date: trade_date.into(),
            state,
        }
    }
}

/// MA 指标参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaParams {
    /// 价格简单移动平均窗口集合。
    pub price_ma_windows: Vec<usize>,
    /// 成交量简单移动平均窗口集合。
    pub volume_ma_windows: Vec<usize>,
    /// EMA 窗口。
    pub ema_window: usize,
}

impl Default for MaParams {
    fn default() -> Self {
        Self {
            price_ma_windows: DEFAULT_PRICE_MA_WINDOWS.to_vec(),
            volume_ma_windows: DEFAULT_VOLUME_MA_WINDOWS.to_vec(),
            ema_window: DEFAULT_EMA_WINDOW,
        }
    }
}

impl MaParams {
    /// 判断参数是否为首版生产 canonical 参数。
    pub fn is_canonical(&self) -> bool {
        self.ema_window == DEFAULT_EMA_WINDOW
            && self.price_ma_windows == DEFAULT_PRICE_MA_WINDOWS
            && self.volume_ma_windows == DEFAULT_VOLUME_MA_WINDOWS
    }

    fn validate(&self) -> Result<(), MaError> {
        if self.ema_window == 0 {
            return Err(MaError::InvalidParams(
                "ema_window must be greater than 0".to_string(),
            ));
        }
        validate_windows("price_ma_windows", &self.price_ma_windows)?;
        validate_windows("volume_ma_windows", &self.volume_ma_windows)?;
        Ok(())
    }
}

fn validate_windows(name: &str, windows: &[usize]) -> Result<(), MaError> {
    if windows.is_empty() {
        return Err(MaError::InvalidParams(format!("{name} must not be empty")));
    }
    let mut previous = None;
    for window in windows {
        if *window == 0 {
            return Err(MaError::InvalidParams(format!(
                "{name} must be greater than 0"
            )));
        }
        if previous.is_some_and(|previous| previous >= *window) {
            return Err(MaError::InvalidParams(format!(
                "{name} must be strictly increasing"
            )));
        }
        previous = Some(*window);
    }
    Ok(())
}

/// 单行 MA 输出。
#[derive(Debug, Clone, PartialEq)]
pub struct MaOutput {
    /// 从输入行复制的交易日期。
    pub trade_date: String,
    /// 3-valid-close simple moving average.
    pub price_ma_3: Option<f64>,
    /// 5-valid-close simple moving average.
    pub price_ma_5: Option<f64>,
    /// 6-valid-close simple moving average.
    pub price_ma_6: Option<f64>,
    /// 10-valid-close simple moving average.
    pub price_ma_10: Option<f64>,
    /// 12-valid-close simple moving average.
    pub price_ma_12: Option<f64>,
    /// 14-valid-close simple moving average.
    pub price_ma_14: Option<f64>,
    /// 20-valid-close simple moving average.
    pub price_ma_20: Option<f64>,
    /// 24-valid-close simple moving average.
    pub price_ma_24: Option<f64>,
    /// 28-valid-close simple moving average.
    pub price_ma_28: Option<f64>,
    /// 30-valid-close simple moving average.
    pub price_ma_30: Option<f64>,
    /// 57-valid-close simple moving average.
    pub price_ma_57: Option<f64>,
    /// 60-valid-close simple moving average.
    pub price_ma_60: Option<f64>,
    /// 114-valid-close simple moving average.
    pub price_ma_114: Option<f64>,
    /// 250-valid-close simple moving average.
    pub price_ma_250: Option<f64>,
    /// `(price_ma_3 + price_ma_6 + price_ma_12 + price_ma_24) / 4`。
    pub price_avg_ma_3_6_12_24: Option<f64>,
    /// `(price_ma_14 + price_ma_28 + price_ma_57 + price_ma_114) / 4`。
    pub price_avg_ma_14_28_57_114: Option<f64>,
    /// 内部 EMA(close, 10) 状态列值。
    pub price_ema1_10_state: Option<f64>,
    /// 业务输出双重 EMA。
    pub price_ema2_10: Option<f64>,
    /// 内部 EMA(EMA(close, 10), 10) 状态列值。
    pub price_ema2_10_state: Option<f64>,
    /// 5-valid-volume simple moving average.
    pub volume_ma_5: Option<f64>,
    /// 10-valid-volume simple moving average.
    pub volume_ma_10: Option<f64>,
    /// 20-valid-volume simple moving average.
    pub volume_ma_20: Option<f64>,
    /// 60-valid-volume simple moving average.
    pub volume_ma_60: Option<f64>,
}

impl MaOutput {
    fn empty(trade_date: impl Into<String>) -> Self {
        Self {
            trade_date: trade_date.into(),
            price_ma_3: None,
            price_ma_5: None,
            price_ma_6: None,
            price_ma_10: None,
            price_ma_12: None,
            price_ma_14: None,
            price_ma_20: None,
            price_ma_24: None,
            price_ma_28: None,
            price_ma_30: None,
            price_ma_57: None,
            price_ma_60: None,
            price_ma_114: None,
            price_ma_250: None,
            price_avg_ma_3_6_12_24: None,
            price_avg_ma_14_28_57_114: None,
            price_ema1_10_state: None,
            price_ema2_10: None,
            price_ema2_10_state: None,
            volume_ma_5: None,
            volume_ma_10: None,
            volume_ma_20: None,
            volume_ma_60: None,
        }
    }

    /// 所有业务指标字段是否均为空。
    pub fn all_business_indicators_null(&self) -> bool {
        [
            self.price_ma_3,
            self.price_ma_5,
            self.price_ma_6,
            self.price_ma_10,
            self.price_ma_12,
            self.price_ma_14,
            self.price_ma_20,
            self.price_ma_24,
            self.price_ma_28,
            self.price_ma_30,
            self.price_ma_57,
            self.price_ma_60,
            self.price_ma_114,
            self.price_ma_250,
            self.volume_ma_5,
            self.volume_ma_10,
            self.volume_ma_20,
            self.volume_ma_60,
        ]
        .iter()
        .all(Option::is_none)
            && self.price_avg_ma_3_6_12_24.is_none()
            && self.price_avg_ma_14_28_57_114.is_none()
            && self.price_ema2_10.is_none()
    }

    /// 读取指定窗口的价格 MA 值。
    pub fn price_ma(&self, window: usize) -> Option<f64> {
        match window {
            3 => self.price_ma_3,
            5 => self.price_ma_5,
            6 => self.price_ma_6,
            10 => self.price_ma_10,
            12 => self.price_ma_12,
            14 => self.price_ma_14,
            20 => self.price_ma_20,
            24 => self.price_ma_24,
            28 => self.price_ma_28,
            30 => self.price_ma_30,
            57 => self.price_ma_57,
            60 => self.price_ma_60,
            114 => self.price_ma_114,
            250 => self.price_ma_250,
            _ => None,
        }
    }

    /// 读取指定窗口的均量 MA 值。
    pub fn volume_ma(&self, window: usize) -> Option<f64> {
        match window {
            5 => self.volume_ma_5,
            10 => self.volume_ma_10,
            20 => self.volume_ma_20,
            60 => self.volume_ma_60,
            _ => None,
        }
    }
}

/// MA 计算错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaError {
    /// 参数值不可用。
    InvalidParams(String),
    /// 输入行没有按交易日期严格升序排列。
    NonIncreasingTradeDate {
        /// 前一行交易日期。
        previous: String,
        /// 当前行交易日期。
        current: String,
    },
    /// 输入价格或状态值不是有限数。
    InvalidPrice,
}

impl fmt::Display for MaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams(message) => write!(f, "invalid MA parameters: {message}"),
            Self::NonIncreasingTradeDate { previous, current } => write!(
                f,
                "input trade_date values must be strictly increasing: previous={previous}, current={current}"
            ),
            Self::InvalidPrice => f.write_str("MA input values and EMA states must be finite"),
        }
    }
}

impl Error for MaError {}

/// 为单只证券的有序时间序列计算 Moving Average 输出。
///
/// # 错误
///
/// 当参数无效、价格无效或输入 `trade_date` 未严格递增时返回错误。
pub fn calculate_ma_series(
    inputs: &[MaInput],
    params: &MaParams,
    previous_state: Option<MaState>,
) -> Result<Vec<MaOutput>, MaError> {
    calculate_ma_series_internal(inputs, params, previous_state, None)
}

/// 为单只证券的有序时间序列计算 Moving Average 输出，并从指定日期后的行延续 EMA 状态。
///
/// SMA 会消费全部输入行；EMA 在 `previous_state.trade_date` 及之前不推进，
/// 从之后的有效 close 开始按上一状态递推。
///
/// # 错误
///
/// 当参数无效、价格无效或输入 `trade_date` 未严格递增时返回错误。
pub fn calculate_ma_series_from_previous_state(
    inputs: &[MaInput],
    params: &MaParams,
    previous_state: Option<MaPreviousState>,
) -> Result<Vec<MaOutput>, MaError> {
    let state = previous_state.as_ref().map(|previous| previous.state);
    let state_date = previous_state
        .as_ref()
        .map(|previous| previous.trade_date.as_str());
    calculate_ma_series_internal(inputs, params, state, state_date)
}

fn calculate_ma_series_internal(
    inputs: &[MaInput],
    params: &MaParams,
    previous_state: Option<MaState>,
    previous_state_date: Option<&str>,
) -> Result<Vec<MaOutput>, MaError> {
    params.validate()?;
    validate_sorted(inputs)?;

    let mut price_sma_operators = params
        .price_ma_windows
        .iter()
        .map(|window| {
            RollingSma::new(*window)
                .map(|sma| (*window, sma))
                .map_err(|message| MaError::InvalidParams(message.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut volume_sma_operators = params
        .volume_ma_windows
        .iter()
        .map(|window| {
            RollingSma::new(*window)
                .map(|sma| (*window, sma))
                .map_err(|message| MaError::InvalidParams(message.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let previous_ema1 = previous_state.map(|state| crate::operators::EmaState {
        value: state.price_ema1_10_state,
    });
    let previous_ema2 = previous_state.map(|state| crate::operators::EmaState {
        value: state.price_ema2_10_state,
    });
    let mut ema1 = SmaSeededEma::new(params.ema_window, previous_ema1)
        .map_err(|message| MaError::InvalidParams(message.to_string()))?;
    let mut ema2 = SmaSeededEma::new(params.ema_window, previous_ema2)
        .map_err(|message| MaError::InvalidParams(message.to_string()))?;
    let mut outputs = Vec::with_capacity(inputs.len());

    for input in inputs {
        if input.close_price.is_some_and(|value| !value.is_finite())
            || input.volume.is_some_and(|value| !value.is_finite())
        {
            return Err(MaError::InvalidPrice);
        }

        let mut output = MaOutput::empty(input.trade_date.clone());
        if let Some(close_price) = input.close_price {
            for (window, sma) in &mut price_sma_operators {
                let value = sma
                    .next(Some(close_price))
                    .map_err(|_| MaError::InvalidPrice)?;
                output.set_price_ma(*window, value);
            }
            let should_advance_ema = previous_state_date
                .map(|state_date| input.trade_date.as_str() > state_date)
                .unwrap_or(true);
            let (ema1_value, ema2_value) = if should_advance_ema {
                let ema1_value = ema1
                    .next(Some(close_price))
                    .map_err(|_| MaError::InvalidPrice)?;
                let ema2_value = ema2.next(ema1_value).map_err(|_| MaError::InvalidPrice)?;
                (ema1_value, ema2_value)
            } else {
                (None, None)
            };
            output.price_avg_ma_3_6_12_24 = average_required(&output, &[3, 6, 12, 24]);
            output.price_avg_ma_14_28_57_114 = average_required(&output, &[14, 28, 57, 114]);
            output.price_ema1_10_state = ema1_value;
            output.price_ema2_10 = ema2_value;
            output.price_ema2_10_state = ema2_value;
        }
        if let Some(volume) = input.volume {
            for (window, sma) in &mut volume_sma_operators {
                let value = sma.next(Some(volume)).map_err(|_| MaError::InvalidPrice)?;
                output.set_volume_ma(*window, value);
            }
        }
        outputs.push(output);
    }

    Ok(outputs)
}

impl MaOutput {
    fn set_price_ma(&mut self, window: usize, value: Option<f64>) {
        match window {
            3 => self.price_ma_3 = value,
            5 => self.price_ma_5 = value,
            6 => self.price_ma_6 = value,
            10 => self.price_ma_10 = value,
            12 => self.price_ma_12 = value,
            14 => self.price_ma_14 = value,
            20 => self.price_ma_20 = value,
            24 => self.price_ma_24 = value,
            28 => self.price_ma_28 = value,
            30 => self.price_ma_30 = value,
            57 => self.price_ma_57 = value,
            60 => self.price_ma_60 = value,
            114 => self.price_ma_114 = value,
            250 => self.price_ma_250 = value,
            _ => {}
        }
    }

    fn set_volume_ma(&mut self, window: usize, value: Option<f64>) {
        match window {
            5 => self.volume_ma_5 = value,
            10 => self.volume_ma_10 = value,
            20 => self.volume_ma_20 = value,
            60 => self.volume_ma_60 = value,
            _ => {}
        }
    }
}

fn validate_sorted(inputs: &[MaInput]) -> Result<(), MaError> {
    let mut previous = None::<&str>;
    for input in inputs {
        if let Some(previous_date) = previous
            && input.trade_date.as_str() <= previous_date
        {
            return Err(MaError::NonIncreasingTradeDate {
                previous: previous_date.to_string(),
                current: input.trade_date.clone(),
            });
        }
        previous = Some(input.trade_date.as_str());
    }
    Ok(())
}

fn average_required(output: &MaOutput, windows: &[usize]) -> Option<f64> {
    let mut sum = 0.0;
    for window in windows {
        sum += output.price_ma(*window)?;
    }
    Some(sum / windows.len() as f64)
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

    fn inputs(values: &[Option<f64>]) -> Vec<MaInput> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| MaInput::new(format!("2026-01-{:02}", index + 1), *value, None))
            .collect()
    }

    fn inputs_with_volume(values: &[Option<f64>], volumes: &[Option<f64>]) -> Vec<MaInput> {
        values
            .iter()
            .zip(volumes)
            .enumerate()
            .map(|(index, (value, volume))| {
                MaInput::new(format!("2026-01-{:02}", index + 1), *value, *volume)
            })
            .collect()
    }

    #[test]
    fn ma_series_calculates_canonical_windows_and_composites() {
        let values = (1..=24).map(|value| Some(value as f64)).collect::<Vec<_>>();
        let outputs = calculate_ma_series(&inputs(&values), &MaParams::default(), None).unwrap();
        let day_24 = outputs.last().unwrap();

        assert_close(day_24.price_ma(3).unwrap(), 23.0);
        assert_close(day_24.price_ma(6).unwrap(), 21.5);
        assert_close(day_24.price_ma(12).unwrap(), 18.5);
        assert_close(day_24.price_ma(24).unwrap(), 12.5);
        assert_close(
            day_24.price_avg_ma_3_6_12_24.unwrap(),
            (23.0 + 21.5 + 18.5 + 12.5) / 4.0,
        );
        assert!(day_24.price_avg_ma_14_28_57_114.is_none());
    }

    #[test]
    fn ma30_first_non_null_occurs_on_30th_valid_close() {
        let values = (1..=30).map(|value| Some(value as f64)).collect::<Vec<_>>();
        let outputs = calculate_ma_series(&inputs(&values), &MaParams::default(), None).unwrap();

        assert!(outputs[28].price_ma(30).is_none());
        assert_close(outputs[29].price_ma(30).unwrap(), 15.5);
    }

    #[test]
    fn ma30_ignores_null_close_when_counting_valid_window() {
        let mut values = (1..=29).map(|value| Some(value as f64)).collect::<Vec<_>>();
        values.push(None);
        values.push(Some(30.0));
        let outputs = calculate_ma_series(&inputs(&values), &MaParams::default(), None).unwrap();

        assert!(outputs[29].price_ma(30).is_none());
        assert_close(outputs[30].price_ma(30).unwrap(), 15.5);
    }

    #[test]
    fn ma_series_outputs_null_row_for_null_close_without_advancing_ema() {
        let mut values = (1..=10).map(|value| Some(value as f64)).collect::<Vec<_>>();
        values.push(None);
        values.push(Some(11.0));
        let outputs = calculate_ma_series(&inputs(&values), &MaParams::default(), None).unwrap();

        assert!(outputs[10].all_business_indicators_null());
        assert!(outputs[10].price_ema1_10_state.is_none());
        assert_close(
            outputs[11].price_ema1_10_state.unwrap(),
            (11.0 - 5.5) * (2.0 / 11.0) + 5.5,
        );
    }

    #[test]
    fn volume_ma_advances_independently_and_treats_zero_as_valid() {
        let close = [Some(1.0), None, Some(2.0), None, Some(3.0)];
        let volume = [Some(10.0), Some(0.0), Some(20.0), None, Some(30.0)];

        let outputs = calculate_ma_series(
            &inputs_with_volume(&close, &volume),
            &MaParams::default(),
            None,
        )
        .unwrap();

        assert!(outputs[1].price_ma(3).is_none());
        assert_eq!(outputs[1].volume_ma(5), None);
        assert_eq!(outputs[4].volume_ma(5), None);
        assert!(outputs[1].all_business_indicators_null());

        let mut more_close = vec![None; 5];
        let more_volume = vec![Some(1.0), Some(0.0), Some(2.0), Some(3.0), Some(4.0)];
        more_close[4] = Some(10.0);
        let outputs = calculate_ma_series(
            &inputs_with_volume(&more_close, &more_volume),
            &MaParams::default(),
            None,
        )
        .unwrap();
        assert_eq!(outputs[4].volume_ma(5), Some(2.0));
    }

    #[test]
    fn close_null_row_can_still_emit_volume_ma() {
        let close = [Some(1.0), Some(2.0), Some(3.0), Some(4.0), None];
        let volume = [Some(10.0), Some(0.0), Some(20.0), Some(30.0), Some(40.0)];

        let outputs = calculate_ma_series(
            &inputs_with_volume(&close, &volume),
            &MaParams::default(),
            None,
        )
        .unwrap();

        assert!(outputs[4].price_ma_3.is_none());
        assert!(outputs[4].price_ema1_10_state.is_none());
        assert_eq!(outputs[4].volume_ma_5, Some(20.0));
        assert!(!outputs[4].all_business_indicators_null());
    }

    #[test]
    fn ema2_first_non_null_occurs_on_19th_valid_close() {
        let values = (1..=19).map(|value| Some(value as f64)).collect::<Vec<_>>();
        let outputs = calculate_ma_series(&inputs(&values), &MaParams::default(), None).unwrap();

        assert!(outputs[17].price_ema2_10.is_none());
        assert!(outputs[18].price_ema2_10.is_some());
        assert_eq!(outputs[18].price_ema2_10, outputs[18].price_ema2_10_state);
    }

    #[test]
    fn ma_series_can_continue_ema_from_previous_state() {
        let state = MaState::new(55.9, 50.0).unwrap();
        let values = [Some(60.0)];

        let outputs =
            calculate_ma_series(&inputs(&values), &MaParams::default(), Some(state)).unwrap();

        assert_close(outputs[0].price_ema1_10_state.unwrap(), 56.64545454545455);
        assert_close(
            outputs[0].price_ema2_10.unwrap(),
            (56.64545454545455 - 50.0) * (2.0 / 11.0) + 50.0,
        );
    }

    #[test]
    fn ma_series_from_previous_state_skips_ema_for_lookback_rows() {
        let state = MaState::new(55.9, 50.0).unwrap();
        let previous = MaPreviousState::new("2026-01-02", state);
        let rows = vec![
            MaInput::new("2026-01-01", Some(10.0), None),
            MaInput::new("2026-01-02", Some(20.0), None),
            MaInput::new("2026-01-03", Some(60.0), None),
        ];

        let outputs =
            calculate_ma_series_from_previous_state(&rows, &MaParams::default(), Some(previous))
                .unwrap();

        assert!(outputs[0].price_ema1_10_state.is_none());
        assert!(outputs[1].price_ema1_10_state.is_none());
        assert_close(outputs[2].price_ema1_10_state.unwrap(), 56.64545454545455);
        assert_eq!(outputs[2].price_ma(3), Some(30.0));
    }

    #[test]
    fn ma_series_rejects_non_increasing_trade_date() {
        let inputs = vec![
            MaInput::new("2026-01-01", Some(1.0), None),
            MaInput::new("2026-01-01", Some(2.0), None),
        ];

        let error = calculate_ma_series(&inputs, &MaParams::default(), None).unwrap_err();

        assert!(matches!(error, MaError::NonIncreasingTradeDate { .. }));
    }

    #[test]
    fn canonical_params_use_ma_57_not_47() {
        let params = MaParams::default();

        assert!(params.price_ma_windows.contains(&57));
        assert!(!params.price_ma_windows.contains(&47));
        assert_eq!(params.volume_ma_windows, DEFAULT_VOLUME_MA_WINDOWS);
        assert!(params.is_canonical());
    }
}
