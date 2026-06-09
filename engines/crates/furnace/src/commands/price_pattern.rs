use furnace_io::{
    DEFAULT_PRICE_PATTERN_CLOSE_COLUMN, DEFAULT_PRICE_PATTERN_HIGH_COLUMN,
    DEFAULT_PRICE_PATTERN_LOW_COLUMN, DEFAULT_PRICE_PATTERN_OUTPUT_TABLE,
    DEFAULT_PRICE_PATTERN_PREV_CLOSE_COLUMN, DEFAULT_PRICE_PATTERN_STREAK_INPUT_TABLE,
    DEFAULT_PRICE_PATTERN_STRUCTURE_INPUT_TABLE, PricePatternRunRequest, PricePatternWriteMode,
};

use crate::cli::CliError;

use super::{
    CommonCommandOptions, CommonCommandOptionsBuilder, is_common_flag, next_flag_value, parse_mode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PricePatternCommandConfig {
    common: CommonCommandOptions<PricePatternWriteMode>,
    structure_input_table: String,
    streak_input_table: String,
    output_table: String,
    high_column: String,
    low_column: String,
    close_column: String,
    prev_close_column: String,
}

impl PricePatternCommandConfig {
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut common = CommonCommandOptionsBuilder::new(PricePatternWriteMode::DryRun);
        let mut structure_input_table = DEFAULT_PRICE_PATTERN_STRUCTURE_INPUT_TABLE.to_string();
        let mut streak_input_table = DEFAULT_PRICE_PATTERN_STREAK_INPUT_TABLE.to_string();
        let mut output_table = DEFAULT_PRICE_PATTERN_OUTPUT_TABLE.to_string();
        let mut high_column = DEFAULT_PRICE_PATTERN_HIGH_COLUMN.to_string();
        let mut low_column = DEFAULT_PRICE_PATTERN_LOW_COLUMN.to_string();
        let mut close_column = DEFAULT_PRICE_PATTERN_CLOSE_COLUMN.to_string();
        let mut prev_close_column = DEFAULT_PRICE_PATTERN_PREV_CLOSE_COLUMN.to_string();

        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            let known_flag = is_common_flag(&flag)
                || matches!(
                    flag.as_str(),
                    "--structure-input-table"
                        | "--streak-input-table"
                        | "--output-table"
                        | "--high-column"
                        | "--low-column"
                        | "--close-column"
                        | "--prev-close-column"
                );
            if !known_flag {
                return Err(CliError::Usage(format!("unknown option: {flag}")));
            }
            let value = next_flag_value(&mut args, &flag)?;
            if common.apply_flag(&flag, value.clone(), |value| {
                parse_mode(value, PricePatternWriteMode::parse)
            })? {
                continue;
            }

            match flag.as_str() {
                "--structure-input-table" => structure_input_table = value,
                "--streak-input-table" => streak_input_table = value,
                "--output-table" => output_table = value,
                "--high-column" => high_column = value,
                "--low-column" => low_column = value,
                "--close-column" => close_column = value,
                "--prev-close-column" => prev_close_column = value,
                _ => unreachable!("flag match is exhaustive"),
            }
        }

        Ok(Self {
            common: common.finish()?,
            structure_input_table,
            streak_input_table,
            output_table,
            high_column,
            low_column,
            close_column,
            prev_close_column,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), CliError> {
        Ok(())
    }

    pub(crate) fn to_request(&self) -> PricePatternRunRequest {
        PricePatternRunRequest {
            request_from: self.common.request_from.clone(),
            request_to: self.common.request_to.clone(),
            symbols: self.common.symbols.clone(),
            run_id: self.common.run_id.clone(),
            mode: self.common.mode,
            structure_input_table: self.structure_input_table.clone(),
            streak_input_table: self.streak_input_table.clone(),
            output_table: self.output_table.clone(),
            high_column: self.high_column.clone(),
            low_column: self.low_column.clone(),
            close_column: self.close_column.clone(),
            prev_close_column: self.prev_close_column.clone(),
            insert_batch_size: self.common.insert_batch_size,
            ..PricePatternRunRequest::default()
        }
    }
}
