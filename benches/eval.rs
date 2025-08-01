#![allow(clippy::missing_assert_message, clippy::unwrap_used)]
#[macro_use]
extern crate criterion;
extern crate loot_condition_interpreter;

use std::str::FromStr;

use criterion::Criterion;
use loot_condition_interpreter::{Expression, GameType, State};

fn generate_active_plugins() -> Vec<String> {
    let mut vec: Vec<String> = (0_u8..255).map(|i| format!("Blank{i}.esm")).collect();
    vec.push("Blank.esm".into());
    vec
}

fn generate_plugin_versions() -> Vec<(String, String)> {
    let mut vec: Vec<(String, String)> = (0_u8..255)
        .map(|i| (format!("Blank{i}.esm"), "5".to_owned()))
        .collect();
    vec.push(("Blank.esm".into(), "5".into()));
    vec
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Expression.eval() file(path)", |b| {
        let state = State::new(GameType::Oblivion, ".".into());
        let expression = Expression::from_str("file(\"Cargo.toml\")").unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });

    c.bench_function("Expression.eval() file(regex)", |b| {
        let state = State::new(GameType::Oblivion, ".".into());
        let expression = Expression::from_str("file(\"Cargo.*\")").unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });

    c.bench_function("Expression.eval() active(path)", |b| {
        let state = State::new(
            GameType::Oblivion,
            "tests/testing-plugins/Oblivion/Data".into(),
        )
        .with_active_plugins(&generate_active_plugins());

        let expression = Expression::from_str("active(\"Blank.esm\")").unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });

    c.bench_function("Expression.eval() active(regex)", |b| {
        let state = State::new(
            GameType::Oblivion,
            "tests/testing-plugins/Oblivion/Data".into(),
        )
        .with_active_plugins(&generate_active_plugins());

        let expression = Expression::from_str("active(\"Blank.*\")").unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });

    c.bench_function("Expression.eval() many()", |b| {
        let state = State::new(GameType::Oblivion, ".".into());
        let expression = Expression::from_str("many(\"Cargo.*\")").unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });

    c.bench_function("Expression.eval() many_active()", |b| {
        let state = State::new(
            GameType::Oblivion,
            "tests/testing-plugins/Oblivion/Data".into(),
        )
        .with_active_plugins(&generate_active_plugins());

        let expression = Expression::from_str("many_active(\"Blank.*\")").unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });

    c.bench_function("Expression.eval() checksum()", |b| {
        let state = State::new(
            GameType::Oblivion,
            "tests/testing-plugins/Oblivion/Data".into(),
        );
        let expression = Expression::from_str("checksum(\"Blank.esm\", 374E2A6F)").unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });

    c.bench_function("Expression.eval() version(plugin)", |b| {
        let state = State::new(
            GameType::Oblivion,
            "tests/testing-plugins/Oblivion/Data".into(),
        )
        .with_plugin_versions(&generate_plugin_versions());

        let expression = Expression::from_str("version(\"Blank.esm\", \"5.0\", ==)").unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });

    c.bench_function("Expression.eval() version(executable)", |b| {
        let state = State::new(GameType::Oblivion, ".".into());
        let expression =
            Expression::from_str("version(\"tests/libloot_win32/loot.dll\", \"0.18.2.0\", ==)")
                .unwrap();

        b.iter(|| {
            assert!(expression.eval(&state).unwrap());
        });
    });
}
criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
