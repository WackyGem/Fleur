//! 简单移动平均算子。

use std::collections::VecDeque;

/// 维护最近 `window` 个有效值的 rolling SMA。
#[derive(Debug, Clone, PartialEq)]
pub struct RollingSma {
    window: usize,
    values: VecDeque<f64>,
    sum: f64,
}

impl RollingSma {
    /// 创建 rolling SMA 算子。
    ///
    /// # 错误
    ///
    /// 当 `window == 0` 时返回错误。
    pub fn new(window: usize) -> Result<Self, &'static str> {
        if window == 0 {
            return Err("SMA window must be greater than 0");
        }
        Ok(Self {
            window,
            values: VecDeque::with_capacity(window),
            sum: 0.0,
        })
    }

    /// 输入下一行值。`None` 不进入窗口，也不改变状态。
    ///
    /// # 错误
    ///
    /// 当输入值不是有限数时返回错误。
    pub fn next(&mut self, value: Option<f64>) -> Result<Option<f64>, &'static str> {
        let Some(value) = value else {
            return Ok(None);
        };
        if !value.is_finite() {
            return Err("SMA input value must be finite");
        }

        self.values.push_back(value);
        self.sum += value;
        while self.values.len() > self.window {
            if let Some(removed) = self.values.pop_front() {
                self.sum -= removed;
            }
        }

        if self.values.len() < self.window {
            Ok(None)
        } else {
            Ok(Some(self.sum / self.window as f64))
        }
    }

    /// 返回窗口长度。
    pub fn window(&self) -> usize {
        self.window
    }

    /// 返回当前有效窗口内的值数量。
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// 判断当前有效窗口是否为空。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// 计算一组输入的 rolling SMA 序列。
///
/// # 错误
///
/// 当窗口非法或输入包含非有限数时返回错误。
pub fn calculate_sma_series(
    values: &[Option<f64>],
    window: usize,
) -> Result<Vec<Option<f64>>, &'static str> {
    let mut sma = RollingSma::new(window)?;
    values.iter().map(|value| sma.next(*value)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_sma_uses_valid_values_only() {
        let values = [Some(1.0), None, Some(3.0), Some(5.0)];

        let outputs = calculate_sma_series(&values, 2).unwrap();

        assert_eq!(outputs, vec![None, None, Some(2.0), Some(4.0)]);
    }

    #[test]
    fn rolling_sma_rejects_zero_window() {
        let error = RollingSma::new(0).unwrap_err();

        assert_eq!(error, "SMA window must be greater than 0");
    }

    #[test]
    fn rolling_sma_rejects_non_finite_input() {
        let mut sma = RollingSma::new(3).unwrap();

        let error = sma.next(Some(f64::NAN)).unwrap_err();

        assert_eq!(error, "SMA input value must be finite");
    }
}
