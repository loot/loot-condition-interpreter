mod error;
mod function;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::str;
use std::sync::{PoisonError, RwLock, RwLockWriteGuard};

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::combinator::map;
use nom::multi::separated_list0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

use error::ParsingError;
pub use error::{Error, MoreDataNeeded, ParsingErrorKind};
use function::Function;

type ParsingResult<'a, T> = IResult<&'a str, T, ParsingError<&'a str>>;

// GameType variants must not change order, as their integer values are used as
// constants in the C API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameType {
    Oblivion,
    Skyrim,
    SkyrimSE,
    SkyrimVR,
    Fallout3,
    FalloutNV,
    Fallout4,
    Fallout4VR,
    Morrowind,
    Starfield,
}

impl GameType {
    fn supports_light_plugins(self) -> bool {
        matches!(
            self,
            GameType::SkyrimSE
                | GameType::SkyrimVR
                | GameType::Fallout4
                | GameType::Fallout4VR
                | GameType::Starfield
        )
    }
}

#[derive(Debug)]
pub struct State {
    game_type: GameType,
    /// Game Data folder path.
    data_path: PathBuf,
    /// Other directories that may contain plugins and other game files, used before data_path and
    /// in the order they're listed.
    additional_data_paths: Vec<PathBuf>,
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
    pub fn new(game_type: GameType, data_path: PathBuf) -> Self {
        State {
            game_type,
            data_path,
            additional_data_paths: Vec::default(),
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
        self.set_plugin_versions(plugin_versions);
        self
    }

    pub fn with_active_plugins<T: AsRef<str>>(mut self, active_plugins: &[T]) -> Self {
        self.set_active_plugins(active_plugins);
        self
    }

    pub fn set_active_plugins<T: AsRef<str>>(&mut self, active_plugins: &[T]) {
        self.active_plugins = active_plugins
            .iter()
            .map(|s| s.as_ref().to_lowercase())
            .collect();
    }

    pub fn set_plugin_versions<T: AsRef<str>, V: ToString>(&mut self, plugin_versions: &[(T, V)]) {
        self.plugin_versions = plugin_versions
            .iter()
            .map(|(p, v)| (p.as_ref().to_lowercase(), v.to_string()))
            .collect();
    }

    pub fn set_cached_crcs<T: AsRef<str>>(
        &mut self,
        plugin_crcs: &[(T, u32)],
    ) -> Result<(), PoisonError<RwLockWriteGuard<HashMap<String, u32>>>> {
        let mut writer = self.crc_cache.write()?;

        writer.deref_mut().clear();
        writer.deref_mut().extend(
            plugin_crcs
                .iter()
                .map(|(p, v)| (p.as_ref().to_lowercase(), *v)),
        );

        Ok(())
    }

    pub fn clear_condition_cache(
        &mut self,
    ) -> Result<(), PoisonError<RwLockWriteGuard<HashMap<Function, bool>>>> {
        self.condition_cache.write().map(|mut c| c.clear())
    }

    pub fn set_additional_data_paths(&mut self, additional_data_paths: Vec<PathBuf>) {
        self.additional_data_paths = additional_data_paths;
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
            .map_err(Error::from)
            .and_then(|(remaining_input, expression)| {
                if remaining_input.is_empty() {
                    Ok(expression)
                } else {
                    Err(Error::UnconsumedInput(remaining_input.to_string()))
                }
            })
    }
}

fn parse_expression(input: &str) -> ParsingResult<Expression> {
    map(
        separated_list0(map_err(whitespace(tag("or"))), CompoundCondition::parse),
        Expression,
    )
    .parse(input)
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

    fn parse(input: &str) -> ParsingResult<CompoundCondition> {
        map(
            separated_list0(map_err(whitespace(tag("and"))), Condition::parse),
            CompoundCondition,
        )
        .parse(input)
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
    InvertedExpression(Expression),
}

impl Condition {
    fn eval(&self, state: &State) -> Result<bool, Error> {
        match self {
            Condition::Function(f) => f.eval(state),
            Condition::InvertedFunction(f) => f.eval(state).map(|r| !r),
            Condition::Expression(e) => e.eval(state),
            Condition::InvertedExpression(e) => e.eval(state).map(|r| !r),
        }
    }

    fn parse(input: &str) -> ParsingResult<Condition> {
        alt((
            map(Function::parse, Condition::Function),
            map(
                preceded(map_err(whitespace(tag("not"))), Function::parse),
                Condition::InvertedFunction,
            ),
            map(
                delimited(
                    map_err(whitespace(tag("("))),
                    parse_expression,
                    map_err(whitespace(tag(")"))),
                ),
                Condition::Expression,
            ),
            map(
                delimited(
                    map_err(preceded(whitespace(tag("not")), whitespace(tag("(")))),
                    parse_expression,
                    map_err(whitespace(tag(")"))),
                ),
                Condition::InvertedExpression,
            ),
        ))
        .parse(input)
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Condition::*;
        match self {
            Function(function) => write!(f, "{}", function),
            InvertedFunction(function) => write!(f, "not {}", function),
            Expression(e) => write!(f, "({})", e),
            InvertedExpression(e) => write!(f, "not ({})", e),
        }
    }
}

fn map_err<'a, O>(
    mut parser: impl Parser<&'a str, Output = O, Error = nom::error::Error<&'a str>>,
) -> impl FnMut(&'a str) -> ParsingResult<'a, O> {
    move |i| parser.parse(i).map_err(nom::Err::convert)
}

fn whitespace<'a, O>(
    parser: impl Fn(&'a str) -> IResult<&'a str, O>,
) -> impl Parser<&'a str, Output = O, Error = nom::error::Error<&'a str>> {
    delimited(space0, parser, space0)
}

#[cfg(test)]
mod tests {
    use crate::function::ComparisonOperator;

    use super::*;

    use std::fs::create_dir;
    use std::str::FromStr;

    fn state<T: Into<PathBuf>>(data_path: T) -> State {
        let data_path = data_path.into();
        if !data_path.exists() {
            create_dir(&data_path).unwrap();
        }

        State {
            game_type: GameType::Oblivion,
            data_path,
            additional_data_paths: Vec::default(),
            active_plugins: HashSet::new(),
            crc_cache: RwLock::default(),
            plugin_versions: HashMap::default(),
            condition_cache: RwLock::default(),
        }
    }

    #[test]
    fn game_type_supports_light_plugins_should_be_true_for_tes5se_tes5vr_fo4_fo4vr_and_starfield() {
        assert!(GameType::SkyrimSE.supports_light_plugins());
        assert!(GameType::SkyrimVR.supports_light_plugins());
        assert!(GameType::Fallout4.supports_light_plugins());
        assert!(GameType::Fallout4VR.supports_light_plugins());
        assert!(GameType::Starfield.supports_light_plugins());
    }

    #[test]
    fn game_type_supports_light_master_should_be_false_for_tes3_to_5_fo3_and_fonv() {
        assert!(!GameType::Morrowind.supports_light_plugins());
        assert!(!GameType::Oblivion.supports_light_plugins());
        assert!(!GameType::Skyrim.supports_light_plugins());
        assert!(!GameType::Fallout3.supports_light_plugins());
        assert!(!GameType::FalloutNV.supports_light_plugins());
    }

    #[test]
    fn expression_from_str_should_error_with_input_on_incomplete_input() {
        let error = Expression::from_str("file(\"Carg").unwrap_err();

        assert_eq!(
            "The parser did not consume the following input: \"file(\"Carg\"",
            error.to_string()
        );
    }

    #[test]
    fn expression_from_str_should_error_with_input_on_invalid_regex() {
        let error = Expression::from_str("file(\"Carg\\.*(\")").unwrap_err();

        assert_eq!(
            "An error was encountered while parsing the expression \"Carg\\.*(\": regex parse error:\n    ^Carg\\.*($\n            ^\nerror: unclosed group",
            error.to_string()
        );
    }

    #[test]
    fn expression_from_str_should_error_with_input_on_invalid_crc() {
        let error = Expression::from_str("checksum(\"Cargo.toml\", DEADBEEFDEAD)").unwrap_err();

        assert_eq!(
            "An error was encountered while parsing the expression \"DEADBEEFDEAD\": number too large to fit in target type",
            error.to_string()
        );
    }

    #[test]
    fn expression_from_str_should_error_with_input_on_directory_regex() {
        let error = Expression::from_str("file(\"targ.*et/\")").unwrap_err();

        assert_eq!(
            "An error was encountered while parsing the expression \"targ.*et/\\\")\": \"targ.*et/\" ends in a directory separator",
            error.to_string()
        );
    }

    #[test]
    fn expression_from_str_should_error_with_input_on_path_outside_game_directory() {
        let error = Expression::from_str("file(\"../../Cargo.toml\")").unwrap_err();

        assert_eq!(
            "An error was encountered while parsing the expression \"../../Cargo.toml\\\")\": \"../../Cargo.toml\" is not in the game directory",
            error.to_string()
        );
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
    fn expression_parse_should_error_if_it_does_not_consume_the_whole_input() {
        let error = Expression::from_str("file(\"Cargo.toml\") foobar").unwrap_err();

        assert_eq!(
            "The parser did not consume the following input: \" foobar\"",
            error.to_string()
        );
    }

    #[test]
    fn expression_parsing_should_ignore_whitespace_between_function_arguments() {
        let is_ok = |s: &str| Expression::from_str(s).is_ok();

        assert!(is_ok("version(\"Cargo.toml\", \"1.2\", ==)"));
        assert!(is_ok(
            "version(\"Unofficial Oblivion Patch.esp\",\"3.4.0\",>=)"
        ));
        assert!(is_ok(
            "version(\"Unofficial Skyrim Patch.esp\", \"2.0\", >=)"
        ));
        assert!(is_ok("version(\"..\\TESV.exe\", \"1.8\", >) and not checksum(\"EternalShineArmorAndWeapons.esp\",3E85A943)"));
        assert!(is_ok("version(\"..\\TESV.exe\",\"1.8\",>) and not checksum(\"EternalShineArmorAndWeapons.esp\",3E85A943)"));
        assert!(is_ok("checksum(\"HM_HotkeyMod.esp\",374C564C)"));
        assert!(is_ok("checksum(\"HM_HotkeyMod.esp\",CF00AFFD)"));
        assert!(is_ok(
            "checksum(\"HM_HotkeyMod.esp\",374C564C) or checksum(\"HM_HotkeyMod.esp\",CF00AFFD)"
        ));
        assert!(is_ok("( checksum(\"HM_HotkeyMod.esp\",374C564C) or checksum(\"HM_HotkeyMod.esp\",CF00AFFD) )"));
        assert!(is_ok("file(\"UFO - Ultimate Follower Overhaul.esp\")"));
        assert!(is_ok("( checksum(\"HM_HotkeyMod.esp\",374C564C) or checksum(\"HM_HotkeyMod.esp\",CF00AFFD) ) and file(\"UFO - Ultimate Follower Overhaul.esp\")"));
        assert!(is_ok(
            "many(\"Deeper Thoughts (\\(Curie\\)|- (Expressive )?Curie)\\.esp\")"
        ));
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
    fn condition_parse_should_handle_an_inverted_function() {
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
    fn condition_parse_should_handle_an_expression_in_parentheses_with_whitespace() {
        let result = Condition::parse("( not file(\"Cargo.toml\") )").unwrap().1;

        match result {
            Condition::Expression(_) => {}
            v => panic!(
                "Expected an expression with two compound conditions, got {:?}",
                v
            ),
        }
    }

    #[test]
    fn condition_parse_should_handle_an_inverted_expression_in_parentheses() {
        let result = Condition::parse("not(not file(\"Cargo.toml\"))").unwrap().1;

        match result {
            Condition::InvertedExpression(_) => {}
            v => panic!(
                "Expected an expression with two compound conditions, got {:?}",
                v
            ),
        }
    }

    #[test]
    fn condition_parse_should_handle_an_inverted_expression_in_parentheses_with_whitespace() {
        let result = Condition::parse("not ( not file(\"Cargo.toml\") )")
            .unwrap()
            .1;

        match result {
            Condition::InvertedExpression(_) => {}
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
    fn condition_eval_should_return_inverse_of_expression_eval_for_a_not_expression_condition() {
        let state = state(".");

        let condition = Condition::InvertedExpression(Expression(vec![CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
        ])]));

        assert!(!condition.eval(&state).unwrap());
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
    fn condition_fmt_should_format_inverted_expression_correctly() {
        let condition = Condition::InvertedExpression(Expression(vec![CompoundCondition(vec![
            Condition::Function(Function::FilePath(PathBuf::from("Cargo.toml"))),
        ])]));

        assert_eq!("not (file(\"Cargo.toml\"))", &format!("{}", condition));
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
    fn compound_condition_eval_should_return_false_on_first_false_condition() {
        let state = state(".");
        let path = "Cargo.toml";

        // If the second function is evaluated, it will result in an error.
        let compound_condition = CompoundCondition(vec![
            Condition::InvertedFunction(Function::Readable(PathBuf::from(path))),
            Condition::Function(Function::ProductVersion(
                PathBuf::from(path),
                "1.0.0".into(),
                ComparisonOperator::Equal,
            )),
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
