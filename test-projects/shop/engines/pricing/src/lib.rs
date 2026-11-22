//! The Pricing engine: what an order costs, from its lines. A line is its
//! quantity times its price, with 5% off from 10 pieces and 10% off from
//! 50; an order above 1000 gets 2% off the whole. Amounts are rounded to
//! cents, half away from zero, as Clarity rounds.

use rust_decimal::{Decimal, RoundingStrategy};
use serde::Deserialize;

clarity_engine::interface!();

/// A line of an order, as the application hands it over.
#[derive(Deserialize)]
struct Line {
    quantity: i64,
    #[serde(with = "rust_decimal::serde::str")]
    price: Decimal,
}

clarity_engine::function!(total, |(lines,): (Vec<Line>,)| Ok(priced(&lines).to_string()));

fn priced(lines: &[Line]) -> Decimal {
    let mut sum = Decimal::ZERO;
    for line in lines {
        let off = match line.quantity {
            50.. => Decimal::new(10, 2),
            10.. => Decimal::new(5, 2),
            _ => Decimal::ZERO,
        };
        sum += Decimal::from(line.quantity) * line.price * (Decimal::ONE - off);
    }
    if sum > Decimal::from(1000) {
        sum *= Decimal::new(98, 2);
    }
    sum.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}
