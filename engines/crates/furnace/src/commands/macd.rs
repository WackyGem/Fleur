use furnace_io::{
    DEFAULT_INPUT_TABLE, DEFAULT_MACD_OUTPUT_TABLE, DEFAULT_MACD_PRICE_COLUMN, MacdRunRequest,
    MacdWriteMode,
};

use crate::cli::CliError;

use super::{
    CommonCommandOptions, CommonCommandOptionsBuilder, is_common_flag, next_flag_value, parse_mode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacdCommandConfig {
    common: CommonCommandOptions<MacdWriteMode>,
    input_table: String,
    output_table: String,
    price_column: String,
}

impl MacdCommandConfig {
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut common = CommonCommandOptionsBuilder::new(MacdWriteMode::DryRun);
        let mut input_table = DEFAULT_INPUT_TABLE.to_string();
        let mut output_table = DEFAULT_MACD_OUTPUT_TABLE.to_string();
        let mut price_column = DEFAULT_MACD_PRICE_COLUMN.to_string();

        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            let known_flag = is_common_flag(&flag)
                || matches!(
                    flag.as_str(),
                    "--input-table" | "--output-table" | "--price-column"
                );
            if !known_flag {
                return Err(CliError::Usage(format!("unknown option: {flag}")));
            }
            let value = next_flag_value(&mut args, &flag)?;
            if common.apply_flag(&flag, value.clone(), |value| {
                parse_mode(value, MacdWriteMode::parse)
            })? {
                continue;
            }

            match flag.as_str() {
                "--input-table" => input_table = value,
                "--output-table" => output_table = value,
                "--price-column" => price_column = value,
                _ => unreachable!("flag match is exhaustive"),
            }
        }

        Ok(Self {
            common: common.finish()?,
            input_table,
            output_table,
            price_column,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), CliError> {
        Ok(())
    }

    pub(crate) fn to_request(&self) -> MacdRunRequest {
        MacdRunRequest {
            request_from: self.common.request_from.clone(),
            request_to: self.common.request_to.clone(),
            symbols: self.common.symbols.clone(),
            run_id: self.common.run_id.clone(),
            mode: self.common.mode,
            input_table: self.input_table.clone(),
            output_table: self.output_table.clone(),
            price_column: self.price_column.clone(),
            insert_batch_size: self.common.insert_batch_size,
            ..MacdRunRequest::default()
        }
    }
}
