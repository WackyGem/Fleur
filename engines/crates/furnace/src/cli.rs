use std::error::Error;
use std::fmt;

use furnace_io::{
    ClickHouseCliExecutor, ClickHouseExecutor, run_boll, run_kdj, run_ma, run_macd,
    run_price_pattern, run_rsi,
};

use crate::commands::{
    BollCommandConfig, KdjCommandConfig, MaCommandConfig, MacdCommandConfig,
    PricePatternCommandConfig, RsiCommandConfig,
};
use crate::output::print_help;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn run(args: impl IntoIterator<Item = String>) -> Result<String, CliError> {
    let mut executor = ClickHouseCliExecutor::from_env();
    run_with_executor(args, &mut executor)
}

fn run_with_executor<E: ClickHouseExecutor>(
    args: impl IntoIterator<Item = String>,
    executor: &mut E,
) -> Result<String, CliError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Err(CliError::Usage("missing command".to_string()));
    };

    match command.as_str() {
        "kdj" => {
            let config = KdjCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_kdj(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "ma" => {
            let config = MaCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_ma(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "rsi" => {
            let config = RsiCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_rsi(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "boll" => {
            let config = BollCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_boll(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "macd" => {
            let config = MacdCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_macd(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "price-pattern" => {
            let config = PricePatternCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_price_pattern(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "--help" | "-h" => {
            print_help();
            Ok(String::new())
        }
        "--version" | "-V" => Ok(format!("furnace {VERSION}")),
        other => Err(CliError::Usage(format!("unknown command: {other}"))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => f.write_str(message),
        }
    }
}

impl Error for CliError {}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;
