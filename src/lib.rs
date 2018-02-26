#[macro_use]
extern crate nom;
extern crate regex;

use regex::Regex;

use std::str;
use std::path::PathBuf;
use nom::{IError, IResult};

#[derive(Debug)]
enum Error {
    ParsingIncomplete,
    ParsingError,
    InvalidPath(PathBuf),
    InvalidRegex(String),
}

impl From<IError> for Error {
    fn from(error: IError) -> Self {
        match error {
            IError::Error(_) => Error::ParsingError,
            _ => Error::ParsingIncomplete,
        }
    }
}

enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

enum Function {
    FilePath(PathBuf),
    FileRegex(Regex),
    ActivePath(PathBuf),
    ActiveRegex(Regex),
    Many(Regex),
    ManyActive(Regex),
    Checksum(PathBuf, u32),
    Version(PathBuf, String, ComparisonOperator),
}

impl Function {
    fn eval(&self) -> Result<bool, Error> {
        // TODO: Handle all variants.
        // TODO: Paths may not lead outside game directory.
        match *self {
            Function::FilePath(ref f) => Ok(f.exists()),
            _ => Ok(false),
        }
    }

    fn parse(input: &str) -> IResult<&str, Function> {
        // TODO: Handle all variants.
        // TODO: Paths may not contain :*?"<>|
        do_parse!(
            input,
            tag!("file(\"") >> path: is_not!("\"") >> tag!("\")")
                >> (Function::FilePath(PathBuf::from(path)))
        )
    }
}

// Compound conditions joined by 'or'
struct Expression(Vec<CompoundCondition>);

impl Expression {
    fn eval(&self) -> Result<bool, Error> {
        for compound_condition in &self.0 {
            if compound_condition.eval()? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn parse(input: &str) -> IResult<&str, Expression> {
        IResult::Done(input, Expression(vec![]))
    }
}

// Conditions joined by 'and'
struct CompoundCondition(Vec<Condition>);

impl CompoundCondition {
    fn eval(&self) -> Result<bool, Error> {
        for condition in &self.0 {
            if !condition.eval()? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn parse(input: &str) -> IResult<&str, CompoundCondition> {
        IResult::Done(input, CompoundCondition(vec![]))
    }
}

enum Condition {
    Function(Function),
    NotFunction(Function),
    Expression(Expression),
}

impl Condition {
    fn eval(&self) -> Result<bool, Error> {
        match *self {
            Condition::Function(ref f) => f.eval(),
            Condition::NotFunction(ref f) => f.eval().map(|r| !r),
            Condition::Expression(ref e) => e.eval(),
        }
    }

    fn parse(input: &str) -> IResult<&str, Condition> {
        let (input1, function) = try_parse!(input, Function::parse);
        IResult::Done(input1, Condition::Function(function))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_parse_should_parse_a_file_function() {
        let result = Function::parse("file(\"Cargo.toml\")").to_result().unwrap();

        match result {
            Function::FilePath(f) => assert_eq!(PathBuf::from("Cargo.toml"), f),
            _ => panic!("Expected a file function"),
        }
    }

    #[test]
    fn function_file_path_eval_should_return_true_if_the_file_exists_relative_to_the_data_path() {
        let function = Function::FilePath(PathBuf::from("Cargo.toml"));

        assert!(function.eval().unwrap());

        unimplemented!("not yet any way to actually specify the data path");
    }

    #[test]
    fn function_file_path_eval_should_return_true_if_given_a_plugin_that_is_ghosted() {
        let function = Function::FilePath(PathBuf::from("test.esp"));

        assert!(function.eval().unwrap());

        unimplemented!("need to add tempdir and create a test.esp.ghost");
    }

    #[test]
    fn function_file_path_eval_should_error_if_the_path_is_outside_game_directory() {
        unimplemented!("to do");
    }

    #[test]
    fn function_file_path_eval_should_return_false_if_the_file_does_not_exist() {
        let function = Function::FilePath(PathBuf::from("missing"));

        assert!(!function.eval().unwrap());
    }

    #[test]
    fn condition_eval_should_return_function_eval_for_a_function_condition() {
        let condition = Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml")));

        assert!(condition.eval().unwrap());

        let condition = Condition::Function(Function::FilePath(PathBuf::from("missing")));

        assert!(!condition.eval().unwrap());
    }

    #[test]
    fn condition_eval_should_return_expression_eval_for_an_expression_condition() {
        let condition = Condition::Expression(Expression(vec![
            CompoundCondition(vec![
                Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
            ]),
        ]));

        assert!(condition.eval().unwrap());
    }

    #[test]
    fn condition_eval_should_return_inverse_of_function_eval_for_a_not_function_condition() {
        let condition = Condition::NotFunction(Function::FilePath(PathBuf::from("Cargo.toml")));

        assert!(!condition.eval().unwrap());

        let condition = Condition::NotFunction(Function::FilePath(PathBuf::from("missing")));

        assert!(condition.eval().unwrap());
    }

    #[test]
    fn compound_condition_eval_should_be_true_if_all_conditions_are_true() {
        let compound_condition = CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
        ]);

        assert!(compound_condition.eval().unwrap());
    }

    #[test]
    fn compound_condition_eval_should_be_false_if_any_condition_is_false() {
        let compound_condition = CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
            Condition::Function(Function::FilePath(PathBuf::from("missing"))),
        ]);

        assert!(!compound_condition.eval().unwrap());
    }

    #[test]
    fn expression_eval_should_be_true_if_any_compound_condition_is_true() {
        let expression = Expression(vec![
            CompoundCondition(vec![
                Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
            ]),
            CompoundCondition(vec![
                Condition::Function(Function::FilePath(PathBuf::from("missing"))),
            ]),
        ]);
        assert!(expression.eval().unwrap());
    }

    #[test]
    fn expression_eval_should_be_false_if_all_compound_conditions_are_false() {
        let expression = Expression(vec![
            CompoundCondition(vec![
                Condition::Function(Function::FilePath(PathBuf::from("missing"))),
            ]),
            CompoundCondition(vec![
                Condition::Function(Function::FilePath(PathBuf::from("missing"))),
            ]),
        ]);
        assert!(!expression.eval().unwrap());
    }
}
