use rust_decimal::{Decimal, RoundingStrategy};

const OUTPUT_SCALE: u32 = 6;

/// API responses always expose money values with a fixed 6-digit fractional
/// scale, regardless of how SQLite normalizes numeric storage internally.
pub fn normalize_amount_output(amount: &str) -> String {
    let (sign, unsigned) = match amount.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", amount),
    };

    let (integer_part, fractional_part) = match unsigned.split_once('.') {
        Some((integer_part, fractional_part)) => (integer_part, fractional_part),
        None => (unsigned, ""),
    };

    let mut normalized =
        String::with_capacity(sign.len() + integer_part.len() + 1 + OUTPUT_SCALE as usize);
    normalized.push_str(sign);
    normalized.push_str(integer_part);
    normalized.push('.');
    normalized.push_str(fractional_part);

    for _ in fractional_part.len()..OUTPUT_SCALE as usize {
        normalized.push('0');
    }

    normalized
}

pub fn compact_decimal_output(value: &str) -> String {
    match value.split_once('.') {
        Some((integer_part, fractional_part)) => {
            let trimmed_fraction = fractional_part.trim_end_matches('0');

            if trimmed_fraction.is_empty() {
                integer_part.to_string()
            } else {
                format!("{integer_part}.{trimmed_fraction}")
            }
        }
        None => value.to_string(),
    }
}

pub fn format_decimal_amount(amount: Decimal) -> String {
    normalize_amount_output(
        &amount
            .round_dp_with_strategy(OUTPUT_SCALE, RoundingStrategy::MidpointAwayFromZero)
            .to_string(),
    )
}

/// Shorthand: format any `Display` value through `normalize_amount_output`.
pub fn fmt_amount(value: &impl std::fmt::Display) -> String {
    normalize_amount_output(&value.to_string())
}

/// Shorthand: format an optional `Display` value through `normalize_amount_output`.
pub fn fmt_opt_amount(value: Option<&impl std::fmt::Display>) -> Option<String> {
    value.map(|v| normalize_amount_output(&v.to_string()))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use super::{compact_decimal_output, format_decimal_amount, normalize_amount_output};

    #[test]
    fn normalizes_integer_amounts_to_fixed_scale() {
        assert_eq!(normalize_amount_output("12"), "12.000000");
        assert_eq!(normalize_amount_output("0"), "0.000000");
    }

    #[test]
    fn normalizes_fractional_amounts_to_fixed_scale() {
        assert_eq!(normalize_amount_output("12.3"), "12.300000");
        assert_eq!(normalize_amount_output("12.3456"), "12.345600");
    }

    #[test]
    fn preserves_exact_scale_amounts() {
        assert_eq!(normalize_amount_output("12.345678"), "12.345678");
    }

    #[test]
    fn normalizes_negative_amounts_to_fixed_scale() {
        assert_eq!(normalize_amount_output("-12"), "-12.000000");
        assert_eq!(normalize_amount_output("-12.3"), "-12.300000");
    }

    #[test]
    fn compacts_decimal_outputs() {
        assert_eq!(compact_decimal_output("12.340000"), "12.34");
        assert_eq!(compact_decimal_output("12.000000"), "12");
        assert_eq!(compact_decimal_output("12.345600"), "12.3456");
        assert_eq!(compact_decimal_output("12"), "12");
    }

    #[test]
    fn formats_decimal_amounts_to_fixed_scale() {
        let value = Decimal::from_str("12.3456789").expect("decimal should parse");

        assert_eq!(format_decimal_amount(value), "12.345679");
    }
}
