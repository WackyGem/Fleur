use std::str;

use crate::FurnaceIoError;
use crate::validation::date_days_since_unix_epoch;

pub(crate) fn read_rowbinary_string<'a>(
    input: &'a [u8],
    cursor: &mut usize,
) -> Result<&'a str, FurnaceIoError> {
    let length = read_rowbinary_var_uint(input, cursor)?;
    let end = cursor
        .checked_add(length)
        .ok_or_else(|| FurnaceIoError::Parse("RowBinary string length overflow".to_string()))?;
    if end > input.len() {
        return Err(FurnaceIoError::Parse(
            "truncated RowBinary string field".to_string(),
        ));
    }
    let value = str::from_utf8(&input[*cursor..end])
        .map_err(|source| FurnaceIoError::Parse(format!("invalid RowBinary UTF-8: {source}")))?;
    *cursor = end;
    Ok(value)
}

fn read_rowbinary_var_uint(input: &[u8], cursor: &mut usize) -> Result<usize, FurnaceIoError> {
    let mut value = 0u64;
    let mut shift = 0;
    loop {
        if *cursor >= input.len() {
            return Err(FurnaceIoError::Parse(
                "truncated RowBinary VarUInt".to_string(),
            ));
        }
        let byte = input[*cursor];
        *cursor += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return usize::try_from(value)
                .map_err(|_| FurnaceIoError::Parse("RowBinary VarUInt too large".to_string()));
        }
        shift += 7;
        if shift >= 64 {
            return Err(FurnaceIoError::Parse(
                "RowBinary VarUInt exceeds u64".to_string(),
            ));
        }
    }
}

pub(crate) fn read_rowbinary_nullable_f64(
    input: &[u8],
    cursor: &mut usize,
) -> Result<Option<f64>, FurnaceIoError> {
    if *cursor >= input.len() {
        return Err(FurnaceIoError::Parse(
            "truncated RowBinary Nullable(Float64) marker".to_string(),
        ));
    }
    let is_null = input[*cursor];
    *cursor += 1;
    match is_null {
        0 => {
            let end = cursor
                .checked_add(8)
                .ok_or_else(|| FurnaceIoError::Parse("RowBinary Float64 overflow".to_string()))?;
            if end > input.len() {
                return Err(FurnaceIoError::Parse(
                    "truncated RowBinary Float64".to_string(),
                ));
            }
            let bytes = input[*cursor..end].try_into().map_err(|_| {
                FurnaceIoError::Parse("invalid RowBinary Float64 width".to_string())
            })?;
            *cursor = end;
            Ok(Some(f64::from_le_bytes(bytes)))
        }
        1 => Ok(None),
        other => Err(FurnaceIoError::Parse(format!(
            "invalid RowBinary Nullable(Float64) marker: {other}"
        ))),
    }
}
pub(crate) fn push_rowbinary_string(bytes: &mut Vec<u8>, value: &str) {
    push_rowbinary_var_uint(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_rowbinary_var_uint(bytes: &mut Vec<u8>, mut value: usize) {
    while value >= 0x80 {
        bytes.push((value as u8) | 0x80);
        value >>= 7;
    }
    bytes.push(value as u8);
}

pub(crate) fn push_rowbinary_date(bytes: &mut Vec<u8>, value: &str) -> Result<(), FurnaceIoError> {
    let days = date_days_since_unix_epoch(value)?;
    bytes.extend_from_slice(&days.to_le_bytes());
    Ok(())
}

pub(crate) fn push_rowbinary_nullable_date(
    bytes: &mut Vec<u8>,
    value: Option<&str>,
) -> Result<(), FurnaceIoError> {
    match value {
        Some(value) => {
            bytes.push(0);
            push_rowbinary_date(bytes, value)?;
        }
        None => bytes.push(1),
    }
    Ok(())
}

pub(crate) fn push_rowbinary_nullable_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(0);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        None => bytes.push(1),
    }
}

pub(crate) fn push_rowbinary_nullable_i8(bytes: &mut Vec<u8>, value: Option<i8>) {
    match value {
        Some(value) => {
            bytes.push(0);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        None => bytes.push(1),
    }
}

pub(crate) fn push_rowbinary_nullable_u16(bytes: &mut Vec<u8>, value: Option<u16>) {
    match value {
        Some(value) => {
            bytes.push(0);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        None => bytes.push(1),
    }
}

pub(crate) fn push_rowbinary_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_rowbinary_bool(bytes: &mut Vec<u8>, value: bool) {
    bytes.push(u8::from(value));
}
