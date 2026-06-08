use furnace_core::{DEFAULT_D_SMOOTHING, DEFAULT_K_SMOOTHING, DEFAULT_RSV_WINDOW, KdjParams};
use furnace_io::{KdjRunRequest, KdjWriteMode};

use crate::cli::CliError;

use super::{
    CommonCommandOptions, CommonCommandOptionsBuilder, is_common_flag, next_flag_value, parse_mode,
    parse_u16_flag,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KdjCommandConfig {
    common: CommonCommandOptions<KdjWriteMode>,
    rsv_window: u16,
    k_smoothing: u16,
    d_smoothing: u16,
}

impl KdjCommandConfig {
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut common = CommonCommandOptionsBuilder::new(KdjWriteMode::DryRun);
        let mut rsv_window = DEFAULT_RSV_WINDOW;
        let mut k_smoothing = DEFAULT_K_SMOOTHING;
        let mut d_smoothing = DEFAULT_D_SMOOTHING;

        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            let known_flag = is_common_flag(&flag)
                || matches!(
                    flag.as_str(),
                    "--rsv-window" | "--k-smoothing" | "--d-smoothing"
                );
            if !known_flag {
                return Err(CliError::Usage(format!("unknown option: {flag}")));
            }
            let value = next_flag_value(&mut args, &flag)?;
            if common.apply_flag(&flag, value.clone(), |value| {
                parse_mode(value, KdjWriteMode::parse)
            })? {
                continue;
            }

            match flag.as_str() {
                "--rsv-window" => rsv_window = parse_u16_flag("--rsv-window", &value)?,
                "--k-smoothing" => k_smoothing = parse_u16_flag("--k-smoothing", &value)?,
                "--d-smoothing" => d_smoothing = parse_u16_flag("--d-smoothing", &value)?,
                _ => unreachable!("flag match is exhaustive"),
            }
        }

        Ok(Self {
            common: common.finish()?,
            rsv_window,
            k_smoothing,
            d_smoothing,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), CliError> {
        let params = KdjParams {
            rsv_window: self.rsv_window,
            k_smoothing: self.k_smoothing,
            d_smoothing: self.d_smoothing,
            ..KdjParams::default()
        };
        if !params.is_canonical() && self.common.mode.writes_applied() {
            return Err(CliError::Runtime(
                "production KDJ writes only allow canonical parameters 9/3/3".to_string(),
            ));
        }

        Ok(())
    }

    pub(crate) fn to_request(&self) -> KdjRunRequest {
        KdjRunRequest {
            request_from: self.common.request_from.clone(),
            request_to: self.common.request_to.clone(),
            symbols: self.common.symbols.clone(),
            run_id: self.common.run_id.clone(),
            mode: self.common.mode,
            params: KdjParams {
                rsv_window: self.rsv_window,
                k_smoothing: self.k_smoothing,
                d_smoothing: self.d_smoothing,
                ..KdjParams::default()
            },
            insert_batch_size: self.common.insert_batch_size,
        }
    }
}
