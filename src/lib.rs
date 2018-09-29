#[macro_use]
extern crate nom;
extern crate regex;

use nom::{IError, IResult};
use std::path::PathBuf;
use std::str;

mod function;
use function::Function;

#[derive(Debug)]
pub enum Error {
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

// Compound conditions joined by 'or'
#[derive(Debug)]
pub struct Expression(Vec<CompoundCondition>);

impl Expression {
    pub fn eval(&self) -> Result<bool, Error> {
        for compound_condition in &self.0 {
            if compound_condition.eval()? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn parse(input: &str) -> IResult<&str, Expression> {
        do_parse!(
            input,
            compound_conditions:
                separated_list_complete!(ws!(tag!("or")), CompoundCondition::parse)
                >> (Expression(compound_conditions))
        )
    }
}

// Conditions joined by 'and'
#[derive(Debug)]
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
        do_parse!(
            input,
            conditions: separated_list_complete!(ws!(tag!("and")), Condition::parse)
                >> (CompoundCondition(conditions))
        )
    }
}

#[derive(Debug)]
enum Condition {
    Function(Function),
    InvertedFunction(Function),
    Expression(Expression),
}

impl Condition {
    fn eval(&self) -> Result<bool, Error> {
        match *self {
            Condition::Function(ref f) => f.eval(),
            Condition::InvertedFunction(ref f) => f.eval().map(|r| !r),
            Condition::Expression(ref e) => e.eval(),
        }
    }

    fn parse(input: &str) -> IResult<&str, Condition> {
        do_parse!(
            input,
            condition:
                alt!(
                    call!(Function::parse) => {
                        |f| Condition::Function(f)
                    } |
                    preceded!(ws!(tag!("not")), call!(Function::parse)) => {
                        |f| Condition::InvertedFunction(f)
                    } |
                    delimited!(tag!("("), call!(Expression::parse), tag!(")")) => {
                        |e| Condition::Expression(e)
                    }
            ) >> (condition)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_parse_should_handle_a_single_compound_condition() {
        let result = Expression::parse("file(\"Cargo.toml\")")
            .to_result()
            .unwrap();

        match result.0.as_slice() {
            [CompoundCondition(_)] => {}
            _ => panic!("Expected an expression with one compound condition"),
        }
    }

    #[test]
    fn expression_parse_should_handle_multiple_compound_conditions() {
        let result = Expression::parse("file(\"Cargo.toml\") or file(\"Cargo.toml\")")
            .to_result()
            .unwrap();

        match result.0.as_slice() {
            [CompoundCondition(_), CompoundCondition(_)] => {}
            v => panic!(
                "Expected an expression with two compound conditions, got {:?}",
                v
            ),
        }
    }

    #[test]
    fn compound_condition_parse_should_handle_a_single_condition() {
        let result = CompoundCondition::parse("file(\"Cargo.toml\")")
            .to_result()
            .unwrap();

        match result.0.as_slice() {
            [Condition::Function(Function::FilePath(f))] => {
                assert_eq!(&PathBuf::from("Cargo.toml"), f)
            }
            v => panic!(
                "Expected an expression with two compound conditions, got {:?}",
                v
            ),
        }
    }

    #[test]
    fn compound_condition_parse_should_handle_multiple_conditions() {
        let result = CompoundCondition::parse("file(\"Cargo.toml\") and file(\"README.md\")")
            .to_result()
            .unwrap();

        match result.0.as_slice() {
            [Condition::Function(Function::FilePath(f1)), Condition::Function(Function::FilePath(f2))] =>
            {
                assert_eq!(&PathBuf::from("Cargo.toml"), f1);
                assert_eq!(&PathBuf::from("README.md"), f2);
            }
            v => panic!(
                "Expected an expression with two compound conditions, got {:?}",
                v
            ),
        }
    }

    #[test]
    fn condition_parse_should_handle_a_function() {
        let result = Condition::parse("file(\"Cargo.toml\")")
            .to_result()
            .unwrap();

        match result {
            Condition::Function(Function::FilePath(f)) => {
                assert_eq!(PathBuf::from("Cargo.toml"), f)
            }
            v => panic!(
                "Expected an expression with two compound conditions, got {:?}",
                v
            ),
        }
    }

    #[test]
    fn condition_parse_should_handle_a_inverted_function() {
        let result = Condition::parse("not file(\"Cargo.toml\")")
            .to_result()
            .unwrap();

        match result {
            Condition::InvertedFunction(Function::FilePath(f)) => {
                assert_eq!(PathBuf::from("Cargo.toml"), f)
            }
            v => panic!(
                "Expected an expression with two compound conditions, got {:?}",
                v
            ),
        }
    }

    #[test]
    fn condition_parse_should_handle_an_expression_in_parentheses() {
        let result = Condition::parse("(not file(\"Cargo.toml\"))")
            .to_result()
            .unwrap();

        match result {
            Condition::Expression(_) => {}
            v => panic!(
                "Expected an expression with two compound conditions, got {:?}",
                v
            ),
        }
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
        let condition = Condition::Expression(Expression(vec![CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
        ])]));

        assert!(condition.eval().unwrap());
    }

    #[test]
    fn condition_eval_should_return_inverse_of_function_eval_for_a_not_function_condition() {
        let condition =
            Condition::InvertedFunction(Function::FilePath(PathBuf::from("Cargo.toml")));

        assert!(!condition.eval().unwrap());

        let condition = Condition::InvertedFunction(Function::FilePath(PathBuf::from("missing")));

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
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("Cargo.toml"),
            ))]),
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("missing"),
            ))]),
        ]);
        assert!(expression.eval().unwrap());
    }

    #[test]
    fn expression_eval_should_be_false_if_all_compound_conditions_are_false() {
        let expression = Expression(vec![
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("missing"),
            ))]),
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("missing"),
            ))]),
        ]);
        assert!(!expression.eval().unwrap());
    }
}
