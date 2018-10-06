extern crate crc;
#[macro_use]
extern crate nom;
extern crate pelite;
extern crate regex;
extern crate unicase;

#[cfg(test)]
extern crate tempfile;

mod function;
mod version;

use std::collections::{HashMap, HashSet};
use std::error;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::RwLock;

use nom::{Err, IResult};

use function::Function;

#[derive(Debug)]
pub enum Error {
    ParsingIncomplete,
    ParsingError,
    PeParsingError,
    IoError(io::Error),
}

impl<I> From<Err<I>> for Error {
    fn from(error: Err<I>) -> Self {
        match error {
            Err::Incomplete(_) => Error::ParsingIncomplete,
            _ => Error::ParsingError,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ParsingIncomplete => write!(f, "More input was expected by the parser"),
            Error::ParsingError => {
                write!(f, "An error was encountered while parsing the expression")
            }
            Error::PeParsingError => write!(
                f,
                "An error was encountered while reading the version of an executable"
            ),
            Error::IoError(e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::IoError(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameType {
    Tes4,
    Tes5,
    Tes5se,
    Tes5vr,
    Fo3,
    Fonv,
    Fo4,
    Fo4vr,
}

impl GameType {
    fn supports_light_plugins(&self) -> bool {
        match self {
            GameType::Tes5se | GameType::Tes5vr | GameType::Fo4 | GameType::Fo4vr => true,
            _ => false,
        }
    }

    fn is_plugin_filename(&self, path: &Path) -> bool {
        match path.extension().and_then(OsStr::to_str) {
            Some("esp") | Some("esm") => true,
            Some("esl") if self.supports_light_plugins() => true,
            Some("ghost") => path
                .file_stem()
                .map(|s| self.is_plugin_filename(Path::new(s)))
                .unwrap_or(false),
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct State {
    game_type: GameType,
    /// Game Data folder path.
    data_path: PathBuf,
    /// Path to the LOOT executable, used to resolve conditions that use the "LOOT" path.
    loot_path: PathBuf,
    /// Lowercased plugin filenames.
    active_plugins: HashSet<String>,
    /// Lowercased paths.
    crc_cache: RwLock<HashMap<String, u32>>,
    /// Lowercased plugin filenames and their versions as found in description fields.
    plugin_versions: HashMap<String, String>,
    /// Conditions that have already been evaluated, and their results.
    condition_cache: RwLock<HashMap<Function, bool>>,
}

impl State {
    pub fn new(game_type: GameType, data_path: PathBuf, loot_path: PathBuf) -> Self {
        State {
            game_type,
            data_path,
            loot_path,
            active_plugins: HashSet::default(),
            crc_cache: RwLock::default(),
            plugin_versions: HashMap::default(),
            condition_cache: RwLock::default(),
        }
    }

    pub fn with_plugin_versions<T: AsRef<str>, V: ToString>(
        mut self,
        plugin_versions: &[(T, V)],
    ) -> Self {
        self.plugin_versions = plugin_versions
            .iter()
            .map(|(p, v)| (p.as_ref().to_lowercase(), v.to_string()))
            .collect();
        self
    }

    pub fn with_active_plugins<T: AsRef<str>>(mut self, active_plugins: &[T]) -> Self {
        self.active_plugins = active_plugins
            .into_iter()
            .map(|s| s.as_ref().to_lowercase())
            .collect();
        self
    }
}

/// Compound conditions joined by 'or'
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Expression(Vec<CompoundCondition>);

impl Expression {
    pub fn eval(&self, state: &State) -> Result<bool, Error> {
        for compound_condition in &self.0 {
            if compound_condition.eval(state)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl str::FromStr for Expression {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_expression(s)
            .map(|(_, expression)| expression)
            .map_err(Error::from)
    }
}

fn parse_expression(input: &str) -> IResult<&str, Expression> {
    do_parse!(
        input,
        compound_conditions: separated_list_complete!(ws!(tag!("or")), CompoundCondition::parse)
            >> (Expression(compound_conditions))
    )
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let strings: Vec<String> = self.0.iter().map(CompoundCondition::to_string).collect();
        write!(f, "{}", strings.join(" or "))
    }
}

/// Conditions joined by 'and'
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
struct CompoundCondition(Vec<Condition>);

impl CompoundCondition {
    fn eval(&self, state: &State) -> Result<bool, Error> {
        for condition in &self.0 {
            if !condition.eval(state)? {
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

impl fmt::Display for CompoundCondition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let strings: Vec<String> = self.0.iter().map(Condition::to_string).collect();
        write!(f, "{}", strings.join(" and "))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Condition {
    Function(Function),
    InvertedFunction(Function),
    Expression(Expression),
}

impl Condition {
    fn eval(&self, state: &State) -> Result<bool, Error> {
        match *self {
            Condition::Function(ref f) => f.eval(state),
            Condition::InvertedFunction(ref f) => f.eval(state).map(|r| !r),
            Condition::Expression(ref e) => e.eval(state),
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
                    delimited!(tag!("("), call!(parse_expression), tag!(")")) => {
                        |e| Condition::Expression(e)
                    }
            ) >> (condition)
        )
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Condition::*;
        match self {
            Function(function) => write!(f, "{}", function),
            InvertedFunction(function) => write!(f, "not {}", function),
            Expression(e) => write!(f, "({})", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::create_dir;
    use std::str::FromStr;

    fn state<T: Into<PathBuf>>(data_path: T) -> State {
        let data_path = data_path.into();
        if !data_path.exists() {
            create_dir(&data_path).unwrap();
        }

        State {
            game_type: GameType::Tes4,
            data_path: data_path,
            loot_path: PathBuf::new(),
            active_plugins: HashSet::new(),
            crc_cache: RwLock::default(),
            plugin_versions: HashMap::default(),
            condition_cache: RwLock::default(),
        }
    }

    #[test]
    fn game_type_supports_light_plugins_should_be_true_for_tes5se_tes5vr_fo4_and_fo4vr() {
        assert!(GameType::Tes5se.supports_light_plugins());
        assert!(GameType::Tes5vr.supports_light_plugins());
        assert!(GameType::Fo4.supports_light_plugins());
        assert!(GameType::Fo4vr.supports_light_plugins());
    }

    #[test]
    fn game_type_supports_light_master_should_be_false_for_tes4_tes5_fo3_and_fonv() {
        assert!(!GameType::Tes4.supports_light_plugins());
        assert!(!GameType::Tes5.supports_light_plugins());
        assert!(!GameType::Fo3.supports_light_plugins());
        assert!(!GameType::Fonv.supports_light_plugins());
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_true_for_esp_for_all_game_types() {
        let filename = Path::new("Blank.esp");

        assert!(GameType::Tes4.is_plugin_filename(filename));
        assert!(GameType::Tes5.is_plugin_filename(filename));
        assert!(GameType::Tes5se.is_plugin_filename(filename));
        assert!(GameType::Tes5vr.is_plugin_filename(filename));
        assert!(GameType::Fo3.is_plugin_filename(filename));
        assert!(GameType::Fonv.is_plugin_filename(filename));
        assert!(GameType::Fo4.is_plugin_filename(filename));
        assert!(GameType::Fo4vr.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_true_for_esm_for_all_game_types() {
        let filename = Path::new("Blank.esm");

        assert!(GameType::Tes4.is_plugin_filename(filename));
        assert!(GameType::Tes5.is_plugin_filename(filename));
        assert!(GameType::Tes5se.is_plugin_filename(filename));
        assert!(GameType::Tes5vr.is_plugin_filename(filename));
        assert!(GameType::Fo3.is_plugin_filename(filename));
        assert!(GameType::Fonv.is_plugin_filename(filename));
        assert!(GameType::Fo4.is_plugin_filename(filename));
        assert!(GameType::Fo4vr.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_true_for_esl_for_tes5se_tes5vr_fo4_and_fo4vr() {
        let filename = Path::new("Blank.esl");

        assert!(GameType::Tes5se.is_plugin_filename(filename));
        assert!(GameType::Tes5vr.is_plugin_filename(filename));
        assert!(GameType::Fo4.is_plugin_filename(filename));
        assert!(GameType::Fo4vr.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_false_for_esl_for_tes4_tes5_fo3_and_fonv() {
        let filename = Path::new("Blank.esl");

        assert!(!GameType::Tes4.is_plugin_filename(filename));
        assert!(!GameType::Tes5.is_plugin_filename(filename));
        assert!(!GameType::Fo3.is_plugin_filename(filename));
        assert!(!GameType::Fonv.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_true_for_esp_dot_ghost_for_all_game_types() {
        let filename = Path::new("Blank.esp.ghost");

        assert!(GameType::Tes4.is_plugin_filename(filename));
        assert!(GameType::Tes5.is_plugin_filename(filename));
        assert!(GameType::Tes5se.is_plugin_filename(filename));
        assert!(GameType::Tes5vr.is_plugin_filename(filename));
        assert!(GameType::Fo3.is_plugin_filename(filename));
        assert!(GameType::Fonv.is_plugin_filename(filename));
        assert!(GameType::Fo4.is_plugin_filename(filename));
        assert!(GameType::Fo4vr.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_true_for_esm_dot_ghost_for_all_game_types() {
        let filename = Path::new("Blank.esm.ghost");

        assert!(GameType::Tes4.is_plugin_filename(filename));
        assert!(GameType::Tes5.is_plugin_filename(filename));
        assert!(GameType::Tes5se.is_plugin_filename(filename));
        assert!(GameType::Tes5vr.is_plugin_filename(filename));
        assert!(GameType::Fo3.is_plugin_filename(filename));
        assert!(GameType::Fonv.is_plugin_filename(filename));
        assert!(GameType::Fo4.is_plugin_filename(filename));
        assert!(GameType::Fo4vr.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_true_for_esl_dot_ghost_for_tes5se_tes5vr_fo4_and_fo4vr(
) {
        let filename = Path::new("Blank.esl.ghost");

        assert!(GameType::Tes5se.is_plugin_filename(filename));
        assert!(GameType::Tes5vr.is_plugin_filename(filename));
        assert!(GameType::Fo4.is_plugin_filename(filename));
        assert!(GameType::Fo4vr.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_false_for_esl_dot_ghost_for_tes4_tes5_fo3_and_fonv() {
        let filename = Path::new("Blank.esl.ghost");

        assert!(!GameType::Tes4.is_plugin_filename(filename));
        assert!(!GameType::Tes5.is_plugin_filename(filename));
        assert!(!GameType::Fo3.is_plugin_filename(filename));
        assert!(!GameType::Fonv.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_false_for_non_esp_esm_esl_for_all_game_types() {
        let filename = Path::new("Blank.txt");

        assert!(!GameType::Tes4.is_plugin_filename(filename));
        assert!(!GameType::Tes5.is_plugin_filename(filename));
        assert!(!GameType::Tes5se.is_plugin_filename(filename));
        assert!(!GameType::Tes5vr.is_plugin_filename(filename));
        assert!(!GameType::Fo3.is_plugin_filename(filename));
        assert!(!GameType::Fonv.is_plugin_filename(filename));
        assert!(!GameType::Fo4.is_plugin_filename(filename));
        assert!(!GameType::Fo4vr.is_plugin_filename(filename));
    }

    #[test]
    fn game_type_is_plugin_filename_should_be_false_for_non_esp_esm_esl_dot_ghost_for_all_game_types(
) {
        let filename = Path::new("Blank.txt.ghost");

        assert!(!GameType::Tes4.is_plugin_filename(filename));
        assert!(!GameType::Tes5.is_plugin_filename(filename));
        assert!(!GameType::Tes5se.is_plugin_filename(filename));
        assert!(!GameType::Tes5vr.is_plugin_filename(filename));
        assert!(!GameType::Fo3.is_plugin_filename(filename));
        assert!(!GameType::Fonv.is_plugin_filename(filename));
        assert!(!GameType::Fo4.is_plugin_filename(filename));
        assert!(!GameType::Fo4vr.is_plugin_filename(filename));
    }

    #[test]
    fn expression_parse_should_handle_a_single_compound_condition() {
        let result = Expression::from_str("file(\"Cargo.toml\")").unwrap();

        match result.0.as_slice() {
            [CompoundCondition(_)] => {}
            _ => panic!("Expected an expression with one compound condition"),
        }
    }

    #[test]
    fn expression_parse_should_handle_multiple_compound_conditions() {
        let result = Expression::from_str("file(\"Cargo.toml\") or file(\"Cargo.toml\")").unwrap();

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
        let result = CompoundCondition::parse("file(\"Cargo.toml\")").unwrap().1;

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
            .unwrap()
            .1;

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
        let result = Condition::parse("file(\"Cargo.toml\")").unwrap().1;

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
        let result = Condition::parse("not file(\"Cargo.toml\")").unwrap().1;

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
        let result = Condition::parse("(not file(\"Cargo.toml\"))").unwrap().1;

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
        let state = state(".");

        let condition = Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml")));

        assert!(condition.eval(&state).unwrap());

        let condition = Condition::Function(Function::FilePath(PathBuf::from("missing")));

        assert!(!condition.eval(&state).unwrap());
    }

    #[test]
    fn condition_eval_should_return_expression_eval_for_an_expression_condition() {
        let state = state(".");

        let condition = Condition::Expression(Expression(vec![CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
        ])]));

        assert!(condition.eval(&state).unwrap());
    }

    #[test]
    fn condition_eval_should_return_inverse_of_function_eval_for_a_not_function_condition() {
        let state = state(".");

        let condition =
            Condition::InvertedFunction(Function::FilePath(PathBuf::from("Cargo.toml")));

        assert!(!condition.eval(&state).unwrap());

        let condition = Condition::InvertedFunction(Function::FilePath(PathBuf::from("missing")));

        assert!(condition.eval(&state).unwrap());
    }

    #[test]
    fn condition_fmt_should_format_function_correctly() {
        let condition = Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml")));

        assert_eq!("file(\"Cargo.toml\")", &format!("{}", condition));
    }

    #[test]
    fn condition_fmt_should_format_inverted_function_correctly() {
        let condition =
            Condition::InvertedFunction(Function::FilePath(PathBuf::from("Cargo.toml")));

        assert_eq!("not file(\"Cargo.toml\")", &format!("{}", condition));
    }

    #[test]
    fn condition_fmt_should_format_expression_correctly() {
        let condition = Condition::Expression(Expression(vec![CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
        ])]));

        assert_eq!("(file(\"Cargo.toml\"))", &format!("{}", condition));
    }

    #[test]
    fn compound_condition_eval_should_be_true_if_all_conditions_are_true() {
        let state = state(".");

        let compound_condition = CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
        ]);

        assert!(compound_condition.eval(&state).unwrap());
    }

    #[test]
    fn compound_condition_eval_should_be_false_if_any_condition_is_false() {
        let state = state(".");

        let compound_condition = CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
            Condition::Function(Function::FilePath(PathBuf::from("missing"))),
        ]);

        assert!(!compound_condition.eval(&state).unwrap());
    }

    #[test]
    fn compound_condition_fmt_should_format_correctly() {
        let compound_condition = CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
            Condition::Function(Function::FilePath(PathBuf::from("missing"))),
        ]);

        assert_eq!(
            "file(\"Cargo.toml\") and file(\"missing\")",
            &format!("{}", compound_condition)
        );

        let compound_condition = CompoundCondition(vec![Condition::Function(Function::FilePath(
            PathBuf::from("Cargo.toml"),
        ))]);

        assert_eq!("file(\"Cargo.toml\")", &format!("{}", compound_condition));
    }

    #[test]
    fn expression_eval_should_be_true_if_any_compound_condition_is_true() {
        let state = state(".");

        let expression = Expression(vec![
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("Cargo.toml"),
            ))]),
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("missing"),
            ))]),
        ]);
        assert!(expression.eval(&state).unwrap());
    }

    #[test]
    fn expression_eval_should_be_false_if_all_compound_conditions_are_false() {
        let state = state(".");

        let expression = Expression(vec![
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("missing"),
            ))]),
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("missing"),
            ))]),
        ]);
        assert!(!expression.eval(&state).unwrap());
    }

    #[test]
    fn expression_fmt_should_format_correctly() {
        let expression = Expression(vec![
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("Cargo.toml"),
            ))]),
            CompoundCondition(vec![Condition::Function(Function::FilePath(
                PathBuf::from("missing"),
            ))]),
        ]);

        assert_eq!(
            "file(\"Cargo.toml\") or file(\"missing\")",
            &format!("{}", expression)
        );

        let expression = Expression(vec![CompoundCondition(vec![Condition::Function(
            Function::FilePath(PathBuf::from("Cargo.toml")),
        )])]);

        assert_eq!("file(\"Cargo.toml\")", &format!("{}", expression));
    }
}
