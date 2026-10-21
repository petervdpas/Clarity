//! Lists of values and random as every runtime gives them (conformance/lists.json).

use clarity_runtime::{Random, at, at_mut};

const CASES: &str = include_str!("../../../conformance/lists.json");

fn file() -> serde_json::Value {
    serde_json::from_str(CASES).unwrap()
}

#[test]
fn an_item_is_read_by_its_position_from_1() {
    for case in file()["items"].as_array().unwrap() {
        let list: Vec<i64> = case["list"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
        let position = case["at"].as_i64().unwrap();
        match (at(&list, position), case.get("gives")) {
            (Ok(value), Some(gives)) => assert_eq!(*value, gives.as_i64().unwrap(), "{case}"),
            (Err(e), None) => assert_eq!(e.to_string(), case["fails"].as_str().unwrap(), "{case}"),
            (got, _) => panic!("{case}: {got:?}"),
        }
    }
}

#[test]
fn an_item_is_set_by_its_position_from_1() {
    let mut list = vec![10i64, 20, 30];
    *at_mut(&mut list, 2).unwrap() = 99;
    assert_eq!(list, [10, 99, 30]);
    assert_eq!(at_mut(&mut list, 4).unwrap_err().to_string(), "there is no item 4 in a list of 3");
}

#[test]
fn random_gives_what_every_runtime_gives_from_a_seed() {
    for case in file()["random"].as_array().unwrap() {
        let mut random = Random::seeded(case["seed"].as_u64().unwrap());
        let max = case["max"].as_i64().unwrap();
        match case.get("gives") {
            Some(gives) => {
                let got: Vec<i64> = gives.as_array().unwrap().iter().map(|_| random.next(max).unwrap()).collect();
                let want: Vec<i64> = gives.as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
                assert_eq!(got, want, "{case}");
            }
            None => assert_eq!(random.next(max).unwrap_err().to_string(), case["fails"].as_str().unwrap()),
        }
    }
}

#[test]
fn random_stays_within_its_highest_number() {
    for _ in 0..200 {
        let n = clarity_runtime::random(6).unwrap();
        assert!((1..=6).contains(&n), "{n}");
    }
}
