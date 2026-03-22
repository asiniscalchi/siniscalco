use rust_decimal::{Decimal, RoundingStrategy};

/// API responses always expose money values with a fixed 8-digit fractional
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

    let mut normalized = String::with_capacity(sign.len() + integer_part.len() + 9);
    normalized.push_str(sign);
    normalized.push_str(integer_part);
    normalized.push('.');
    normalized.push_str(fractional_part);

    for _ in fractional_part.len()..8 {
        normalized.push('0');
    }

    normalized
}

pub fn format_decimal_amount(amount: Decimal) -> String {
    normalize_amount_output(
        &amount
            .round_dp_with_strategy(8, RoundingStrategy::MidpointAwayFromZero)
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use super::{format_decimal_amount, normalize_amount_output};

    #[test]
    fn normalizes_integer_amounts_to_fixed_scale() {
        assert_eq!(normalize_amount_output("12"), "12.00000000");
        assert_eq!(normalize_amount_output("0"), "0.00000000");
    }

    #[test]
    fn normalizes_fractional_amounts_to_fixed_scale() {
        assert_eq!(normalize_amount_output("12.3"), "12.30000000");
        assert_eq!(normalize_amount_output("12.3456"), "12.34560000");
    }

    #[test]
    fn preserves_exact_scale_amounts() {
        assert_eq!(normalize_amount_output("12.34567890"), "12.34567890");
    }

    #[test]
    fn normalizes_negative_amounts_to_fixed_scale() {
        assert_eq!(normalize_amount_output("-12"), "-12.00000000");
        assert_eq!(normalize_amount_output("-12.3"), "-12.30000000");
    }

    #[test]
    fn formats_decimal_amounts_to_fixed_scale() {
        let value = Decimal::from_str("12.345678901").expect("decimal should parse");

        assert_eq!(format_decimal_amount(value), "12.34567890");
    }
}
