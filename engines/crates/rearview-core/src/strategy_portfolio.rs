use chrono::{DateTime, Utc};

const PORTFOLIO_CODE_PREFIX: &str = "SP";
const PORTFOLIO_CODE_SUFFIX_LEN: usize = 5;

pub fn new_strategy_portfolio_id() -> String {
    ulid::Ulid::new().to_string()
}

pub fn new_strategy_portfolio_daily_run_id() -> String {
    ulid::Ulid::new().to_string()
}

pub fn build_portfolio_code(now: DateTime<Utc>, entropy: ulid::Ulid) -> String {
    let date = now.format("%Y%m%d");
    let entropy = entropy.to_string();
    let suffix = &entropy[entropy.len() - PORTFOLIO_CODE_SUFFIX_LEN..];
    format!("{PORTFOLIO_CODE_PREFIX}-{date}-{suffix}")
}

pub fn new_portfolio_code(now: DateTime<Utc>) -> String {
    build_portfolio_code(now, ulid::Ulid::new())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{
        build_portfolio_code, new_strategy_portfolio_daily_run_id, new_strategy_portfolio_id,
    };

    #[test]
    fn build_portfolio_code_should_use_prefix_date_and_suffix() {
        let now = Utc.with_ymd_and_hms(2026, 6, 24, 9, 30, 0).unwrap();
        let entropy = ulid::Ulid::from_string("01J1X7W4F6T2C8A9MZQ1P6N3BY").unwrap();

        let code = build_portfolio_code(now, entropy);

        assert_eq!(code, "SP-20260624-6N3BY");
    }

    #[test]
    fn new_strategy_portfolio_id_should_return_ulid_string() {
        let id = new_strategy_portfolio_id();

        assert_eq!(id.len(), 26);
    }

    #[test]
    fn new_strategy_portfolio_daily_run_id_should_return_ulid_string() {
        let id = new_strategy_portfolio_daily_run_id();

        assert_eq!(id.len(), 26);
    }
}
