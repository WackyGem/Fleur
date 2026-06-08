//! SMA 启动的指数移动平均算子。

use std::collections::VecDeque;

/// 可用于延续递推的 EMA 状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmaState {
    /// 最近一条有效 EMA。
    pub value: f64,
}

impl EmaState {
    /// 创建 EMA 状态。
    ///
    /// # 错误
    ///
    /// 当 `value` 不是有限数时返回错误。
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if !value.is_finite() {
            return Err("EMA state value must be finite");
        }
        Ok(Self { value })
    }
}

/// 前 `window` 个有效值以 SMA 启动，之后递推的 EMA 算子。
#[derive(Debug, Clone, PartialEq)]
pub struct SmaSeededEma {
    window: usize,
    alpha: f64,
    state: Option<EmaState>,
    seed_values: VecDeque<f64>,
    seed_sum: f64,
}

impl SmaSeededEma {
    /// 创建 SMA 启动 EMA 算子。
    ///
    /// 传入 `previous_state` 时直接进入递推阶段，不再重新收集启动窗口。
    ///
    /// # 错误
    ///
    /// 当 `window == 0` 或状态值不是有限数时返回错误。
    pub fn new(window: usize, previous_state: Option<EmaState>) -> Result<Self, &'static str> {
        if window == 0 {
            return Err("EMA window must be greater than 0");
        }
        if let Some(state) = previous_state
            && !state.value.is_finite()
        {
            return Err("EMA state value must be finite");
        }
        Ok(Self {
            window,
            alpha: 2.0 / (window as f64 + 1.0),
            state: previous_state,
            seed_values: VecDeque::with_capacity(window),
            seed_sum: 0.0,
        })
    }

    /// 输入下一行值。`None` 不进入启动窗口，也不推进递推状态。
    ///
    /// # 错误
    ///
    /// 当输入值不是有限数时返回错误。
    pub fn next(&mut self, value: Option<f64>) -> Result<Option<f64>, &'static str> {
        let Some(value) = value else {
            return Ok(None);
        };
        if !value.is_finite() {
            return Err("EMA input value must be finite");
        }

        if let Some(previous) = self.state {
            let next_value = self.alpha * value + (1.0 - self.alpha) * previous.value;
            self.state = Some(EmaState { value: next_value });
            return Ok(Some(next_value));
        }

        self.seed_values.push_back(value);
        self.seed_sum += value;
        if self.seed_values.len() < self.window {
            return Ok(None);
        }

        let initial = self.seed_sum / self.window as f64;
        self.seed_values.clear();
        self.seed_sum = 0.0;
        self.state = Some(EmaState { value: initial });
        Ok(Some(initial))
    }

    /// 返回当前递推状态。
    pub fn state(&self) -> Option<EmaState> {
        self.state
    }

    /// 返回窗口长度。
    pub fn window(&self) -> usize {
        self.window
    }

    /// 返回平滑系数。
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

/// 计算一组输入的 SMA 启动 EMA 序列。
///
/// # 错误
///
/// 当窗口非法、状态非法或输入包含非有限数时返回错误。
pub fn calculate_sma_seeded_ema_series(
    values: &[Option<f64>],
    window: usize,
    previous_state: Option<EmaState>,
) -> Result<Vec<Option<f64>>, &'static str> {
    let mut ema = SmaSeededEma::new(window, previous_state)?;
    values.iter().map(|value| ema.next(*value)).collect()
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

    #[test]
    fn sma_seeded_ema_starts_from_initial_sma() {
        let values = (1..=11).map(|value| Some(value as f64)).collect::<Vec<_>>();

        let outputs = calculate_sma_seeded_ema_series(&values, 10, None).unwrap();

        assert_eq!(outputs[8], None);
        assert_close(outputs[9].unwrap(), 5.5);
        assert_close(outputs[10].unwrap(), (11.0 - 5.5) * (2.0 / 11.0) + 5.5);
    }

    #[test]
    fn sma_seeded_ema_does_not_advance_on_null() {
        let values = [Some(10.0), None, Some(20.0), Some(30.0)];

        let outputs = calculate_sma_seeded_ema_series(&values, 2, None).unwrap();

        assert_eq!(outputs[0], None);
        assert_eq!(outputs[1], None);
        assert_close(outputs[2].unwrap(), 15.0);
        assert_close(outputs[3].unwrap(), 25.0);
    }

    #[test]
    fn sma_seeded_ema_can_continue_from_previous_state() {
        let previous = EmaState::new(55.9).unwrap();
        let mut ema = SmaSeededEma::new(10, Some(previous)).unwrap();

        let output = ema.next(Some(60.0)).unwrap().unwrap();

        assert_close(output, 56.64545454545455);
        assert_eq!(ema.state().unwrap().value, output);
    }

    #[test]
    fn sma_seeded_ema_rejects_non_finite_input() {
        let mut ema = SmaSeededEma::new(10, None).unwrap();

        let error = ema.next(Some(f64::INFINITY)).unwrap_err();

        assert_eq!(error, "EMA input value must be finite");
    }
}
