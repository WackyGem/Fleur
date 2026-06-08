//! Rolling population standard deviation operator.

use std::collections::VecDeque;

const VARIANCE_CLAMP_EPSILON: f64 = 1e-12;

/// Maintains the most recent `window` valid values for population standard deviation.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingStdDev {
    window: usize,
    values: VecDeque<f64>,
    sum: f64,
    sum_sq: f64,
}

/// Maintains one window and returns both rolling mean and population standard deviation.
#[derive(Debug, Clone, PartialEq)]
pub struct RollingMeanStdDev {
    window: usize,
    values: VecDeque<f64>,
    sum: f64,
    sum_sq: f64,
}

impl RollingStdDev {
    /// Create a rolling standard deviation operator.
    ///
    /// # Errors
    ///
    /// Returns an error when `window == 0`.
    pub fn new(window: usize) -> Result<Self, &'static str> {
        if window == 0 {
            return Err("standard deviation window must be greater than 0");
        }
        Ok(Self {
            window,
            values: VecDeque::with_capacity(window),
            sum: 0.0,
            sum_sq: 0.0,
        })
    }

    /// Push the next value. `None` does not enter the window or change state.
    ///
    /// # Errors
    ///
    /// Returns an error when input is non-finite or variance is invalid.
    pub fn next(&mut self, value: Option<f64>) -> Result<Option<f64>, &'static str> {
        let Some(value) = value else {
            return Ok(None);
        };
        if !value.is_finite() {
            return Err("standard deviation input value must be finite");
        }

        self.values.push_back(value);
        self.sum += value;
        self.sum_sq += value * value;
        while self.values.len() > self.window {
            if let Some(removed) = self.values.pop_front() {
                self.sum -= removed;
                self.sum_sq -= removed * removed;
            }
        }

        if self.values.len() < self.window {
            return Ok(None);
        }

        population_stddev(self.sum, self.sum_sq, self.window, &self.values).map(Some)
    }

    /// Return the configured window length.
    pub fn window(&self) -> usize {
        self.window
    }

    /// Return the number of valid values currently in the window.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Return true when the valid-value window is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl RollingMeanStdDev {
    /// Create a rolling mean/stddev operator.
    ///
    /// # Errors
    ///
    /// Returns an error when `window == 0`.
    pub fn new(window: usize) -> Result<Self, &'static str> {
        if window == 0 {
            return Err("mean/stddev window must be greater than 0");
        }
        Ok(Self {
            window,
            values: VecDeque::with_capacity(window),
            sum: 0.0,
            sum_sq: 0.0,
        })
    }

    /// Push the next value. `None` does not enter the window or change state.
    ///
    /// # Errors
    ///
    /// Returns an error when input is non-finite or variance is invalid.
    pub fn next(&mut self, value: Option<f64>) -> Result<Option<(f64, f64)>, &'static str> {
        let Some(value) = value else {
            return Ok(None);
        };
        if !value.is_finite() {
            return Err("mean/stddev input value must be finite");
        }

        self.values.push_back(value);
        self.sum += value;
        self.sum_sq += value * value;
        while self.values.len() > self.window {
            if let Some(removed) = self.values.pop_front() {
                self.sum -= removed;
                self.sum_sq -= removed * removed;
            }
        }

        if self.values.len() < self.window {
            return Ok(None);
        }

        let mean = self.sum / self.window as f64;
        let stddev = population_stddev(self.sum, self.sum_sq, self.window, &self.values)?;
        Ok(Some((mean, stddev)))
    }

    /// Return the configured window length.
    pub fn window(&self) -> usize {
        self.window
    }

    /// Return the number of valid values currently in the window.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Return true when the valid-value window is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

fn population_stddev(
    sum: f64,
    sum_sq: f64,
    window: usize,
    values: &VecDeque<f64>,
) -> Result<f64, &'static str> {
    let window = window as f64;
    let mean = sum / window;
    let mut variance = (sum_sq / window) - (mean * mean);
    if variance.abs() <= VARIANCE_CLAMP_EPSILON && values.iter().any(|value| *value != mean) {
        variance = stable_population_variance(values, mean, window);
    } else if variance < 0.0 {
        if variance >= -VARIANCE_CLAMP_EPSILON {
            variance = 0.0;
        } else {
            variance = stable_population_variance(values, mean, window);
            if variance < 0.0 {
                if variance >= -VARIANCE_CLAMP_EPSILON {
                    variance = 0.0;
                } else {
                    return Err("standard deviation variance must not be materially negative");
                }
            }
        }
    }
    Ok(variance.sqrt())
}

fn stable_population_variance(values: &VecDeque<f64>, mean: f64, window: f64) -> f64 {
    values
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>()
        / window
}

/// Calculate a rolling population standard deviation series.
///
/// # Errors
///
/// Returns an error when the window is invalid or input contains non-finite values.
pub fn calculate_stddev_series(
    values: &[Option<f64>],
    window: usize,
) -> Result<Vec<Option<f64>>, &'static str> {
    let mut stddev = RollingStdDev::new(window)?;
    values.iter().map(|value| stddev.next(*value)).collect()
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
    fn rolling_stddev_uses_population_variance() {
        let values = [Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(5.0)];

        let outputs = calculate_stddev_series(&values, 5).unwrap();

        assert_close(outputs[4].unwrap(), 2.0_f64.sqrt());
    }

    #[test]
    fn rolling_stddev_uses_valid_values_only() {
        let values = [Some(1.0), None, Some(3.0), Some(5.0)];

        let outputs = calculate_stddev_series(&values, 2).unwrap();

        assert_eq!(outputs[0], None);
        assert_eq!(outputs[1], None);
        assert_close(outputs[2].unwrap(), 1.0);
        assert_close(outputs[3].unwrap(), 1.0);
    }

    #[test]
    fn rolling_stddev_returns_zero_for_equal_values() {
        let values = [Some(7.0), Some(7.0), Some(7.0)];

        let outputs = calculate_stddev_series(&values, 3).unwrap();

        assert_eq!(outputs[2], Some(0.0));
    }

    #[test]
    fn rolling_stddev_rejects_zero_window() {
        let error = RollingStdDev::new(0).unwrap_err();

        assert_eq!(error, "standard deviation window must be greater than 0");
    }

    #[test]
    fn rolling_stddev_rejects_non_finite_input() {
        let mut stddev = RollingStdDev::new(3).unwrap();

        let error = stddev.next(Some(f64::INFINITY)).unwrap_err();

        assert_eq!(error, "standard deviation input value must be finite");
    }

    #[test]
    fn rolling_mean_stddev_returns_mean_and_population_stddev() {
        let values = [Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(5.0)];
        let mut operator = RollingMeanStdDev::new(5).unwrap();

        let outputs = values
            .iter()
            .map(|value| operator.next(*value))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let (mean, stddev) = outputs[4].unwrap();
        assert_close(mean, 3.0);
        assert_close(stddev, 2.0_f64.sqrt());
    }

    #[test]
    fn rolling_mean_stddev_does_not_advance_on_null() {
        let values = [Some(1.0), None, Some(3.0)];
        let mut operator = RollingMeanStdDev::new(2).unwrap();

        let outputs = values
            .iter()
            .map(|value| operator.next(*value))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(outputs[1], None);
        let (mean, stddev) = outputs[2].unwrap();
        assert_close(mean, 2.0);
        assert_close(stddev, 1.0);
    }

    #[test]
    fn rolling_mean_stddev_falls_back_when_fast_variance_cancels() {
        let values = [
            Some(1_000_000_000.0),
            Some(1_000_000_001.0),
            Some(1_000_000_002.0),
        ];
        let mut operator = RollingMeanStdDev::new(3).unwrap();

        let outputs = values
            .iter()
            .map(|value| operator.next(*value))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let (mean, stddev) = outputs[2].unwrap();
        assert_close(mean, 1_000_000_001.0);
        assert_close(stddev, (2.0_f64 / 3.0).sqrt());
    }
}
