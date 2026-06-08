mod boll;
mod kdj;
mod ma;
mod rsi;

pub(crate) use boll::BollCommandConfig;
pub(crate) use kdj::KdjCommandConfig;
pub(crate) use ma::MaCommandConfig;
pub(crate) use rsi::RsiCommandConfig;

use crate::cli::CliError;

fn parse_symbols(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_u16_flag(flag: &str, value: &str) -> Result<u16, CliError> {
    value
        .parse::<u16>()
        .map_err(|_| CliError::Usage(format!("{flag} must be a positive integer")))
}

fn parse_usize_flag(flag: &str, value: &str) -> Result<usize, CliError> {
    value
        .parse::<usize>()
        .map_err(|_| CliError::Usage(format!("{flag} must be a positive integer")))
}
