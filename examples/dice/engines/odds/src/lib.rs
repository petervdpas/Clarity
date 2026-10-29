//! The Odds engine: rolls `dice` dice `rolls` times and counts how often
//! each total comes up, from the lowest (all ones) to the highest (all
//! sixes). A million rolls take a while, so it says how far it is, and the
//! window shows a bar with Stop.

clarity_engine::interface!();

/// How many rolls go by between two reports of how far it is.
const STEP: i64 = 10_000;

clarity_engine::function!(simulate, |(dice, rolls): (i64, i64)| {
    if !(1..=6).contains(&dice) {
        return Err(format!("{dice} dice: from 1 to 6"));
    }
    if rolls < 1 {
        return Err(format!("{rolls} rolls: 1 or more"));
    }
    let mut counts = vec![0i64; (dice * 5 + 1) as usize];
    for roll in 1..=rolls {
        let total: i64 = (0..dice).map(|_| (clarity_engine::random() % 6) as i64 + 1).sum();
        counts[(total - dice) as usize] += 1;
        if roll % STEP == 0 || roll == rolls {
            clarity_engine::progress(roll, rolls);
        }
    }
    Ok(counts)
});
