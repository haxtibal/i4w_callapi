use crate::icinga::{ExitCode, IcingaTermination};
use crate::ps;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

type EmptyObject = HashMap<(), ()>;

pub type CheckerResponseBody = HashMap<String, CheckerResult>;

#[derive(PartialEq, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Argument {
    RealArgument(String),
    DummyArgument(bool),
}

#[derive(PartialEq, Debug, Deserialize)]
#[serde(untagged)]
pub enum Exitcode {
    Executed(i32),
    NotExecuted(EmptyObject),
}

#[derive(PartialEq, Debug, Deserialize)]
#[serde(untagged)]
pub enum Perfdata {
    Single(String),
    Multiple(Vec<String>),
    None(EmptyObject),
}

#[derive(PartialEq, Debug, Deserialize)]
pub struct CheckerResult {
    pub exitcode: Exitcode,
    pub checkresult: String,
    pub perfdata: Perfdata,
}

#[derive(PartialEq, Debug, Serialize)]
pub struct CommandArguments(IndexMap<String, ps::CliArgument>);

impl std::convert::From<&[String]> for CommandArguments {
    fn from(args: &[String]) -> Self {
        let mut command_map: IndexMap<String, ps::CliArgument> = IndexMap::new();
        let mut arg_value;

        for idx in 0..args.len() {
            if args[idx].starts_with('-') {
                if idx + 1 < args.len() {
                    let next_value = &args[idx + 1];
                    if next_value.starts_with('-') {
                        arg_value = ps::CliArgument::Bool(true);
                    } else {
                        arg_value = ps::from_str(next_value).unwrap()
                    }
                } else {
                    arg_value = ps::CliArgument::Bool(true);
                }
                command_map.insert(args[idx].replace("-", ""), arg_value);
            } else {
                continue;
            }
        }
        CommandArguments(command_map)
    }
}

impl Perfdata {
    fn valid(&self) -> bool {
        match self {
            Perfdata::None(_) => false,
            Perfdata::Single(single_perfdata) => !single_perfdata.is_empty(),
            Perfdata::Multiple(multiple_perfdatas) => !multiple_perfdatas.is_empty(),
        }
    }
}

impl fmt::Display for Perfdata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Perfdata::Single(single_perfdata) => {
                write!(f, "{}", single_perfdata)
            }
            Perfdata::Multiple(multiple_perfdatas) => {
                write!(f, "{}", multiple_perfdatas.join(""))
            }
            _ => Ok(()),
        }
    }
}

impl fmt::Display for CheckerResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let icinga_cr: String = self.checkresult.replace("\r\n", "\n");
        if self.perfdata.valid() {
            write!(
                f,
                "{}",
                [icinga_cr, format!("{}", self.perfdata)].join(" | ")
            )
        } else {
            write!(f, "{}", icinga_cr)
        }
    }
}

impl IcingaTermination for CheckerResult {
    fn exitcode(&self) -> ExitCode {
        match self.exitcode {
            Exitcode::Executed(code) => ExitCode::from_i32(code),
            Exitcode::NotExecuted(_) => ExitCode::Unknown,
        }
    }

    fn report(&self) {
        println!("{}", self);
        std::process::exit(self.exitcode() as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::{Argument, CheckerResult, CommandArguments, EmptyObject, Exitcode, Perfdata};
    use crate::ps::{CliArgument, Number};
    use serde_json;
    use std::collections::HashMap;

    #[test]
    fn test_commandarguments_from_into() {
        let args = vec![String::from("-foo"), String::from("bar")];
        let cmdargs = CommandArguments::from(&*args);
        assert_eq!(
            cmdargs.0.get("foo").unwrap(),
            &CliArgument::String("bar".to_owned())
        );

        let cmdargs: CommandArguments = args.as_slice().into();
        assert_eq!(
            cmdargs.0.get("foo").unwrap(),
            &CliArgument::String("bar".to_owned())
        );
    }

    #[test]
    fn test_serialize_commandarguments() {
        // positional arguments are not supported
        let args = vec![
            String::from("foo"),
            String::from("bar"),
            String::from("baz"),
        ];
        let cmdargs = CommandArguments::from(args.as_slice());
        assert_eq!(cmdargs.0.len(), 0);

        // parameters with arguments are inserted as key value pairs
        let args = vec![
            String::from("-Warning"),
            String::from("0"),
            String::from("-Critical"),
            String::from("1"),
        ];
        let cmdargs = CommandArguments::from(args.as_slice());
        assert_eq!(cmdargs.0.len(), 2);
        assert_eq!(
            cmdargs.0.get("Warning").unwrap(),
            &CliArgument::Number(Number::PosInt(0))
        );
        assert_eq!(
            cmdargs.0.get("Critical").unwrap(),
            &CliArgument::Number(Number::PosInt(1))
        );

        // switch arguments can be interleaved anywhere, fake value True is inserted
        let args = vec![
            String::from("-Warning"),
            String::from("0"),
            String::from("-switch"),
            String::from("-Critical"),
            String::from("1"),
        ];
        let cmdargs = CommandArguments::from(args.as_slice());
        assert_eq!(cmdargs.0.len(), 3);
        assert_eq!(
            cmdargs.0.get("Warning").unwrap(),
            &CliArgument::Number(Number::PosInt(0))
        );
        assert_eq!(
            cmdargs.0.get("Critical").unwrap(),
            &CliArgument::Number(Number::PosInt(1))
        );
        assert_eq!(cmdargs.0.get("switch").unwrap(), &CliArgument::Bool(true));
    }

    #[test]
    fn test_serialize_arglist() {
        let mut outer = HashMap::new();
        outer.insert("arg1", Argument::RealArgument(String::from("bla")));
        outer.insert("arg2", Argument::DummyArgument(true));
        println!("{}", serde_json::to_string(&outer).unwrap());
    }

    #[test]
    fn test_deserialize_body() {
        let data = r#"{"Invoke-Foo":{"exitcode":0,"checkresult":"[OK] Check package \"Bar\"","perfdata":"\u0027baz\u0027=158;; "}}"#;
        let value: HashMap<String, CheckerResult> = serde_json::from_str(data).unwrap();
        let inner_value = value.values().next().unwrap();
        assert_eq!(inner_value.exitcode, Exitcode::Executed(0));
        assert_eq!(
            inner_value.checkresult,
            String::from("[OK] Check package \"Bar\"")
        );
        assert_eq!(
            inner_value.perfdata,
            Perfdata::Single(String::from("'baz'=158;; "))
        );
    }

    #[test]
    fn test_deserialize_checker_result() {
        let data = r#"{"exitcode":0,"checkresult":"[OK] Check package \"Bar\"","perfdata":["\u0027baz\u0027=158;; ", "\u0027qux\u0027=158;; "]}"#;
        let value: CheckerResult = serde_json::from_str(data).unwrap();
        assert_eq!(value.exitcode, Exitcode::Executed(0));
        assert_eq!(
            value.checkresult,
            String::from("[OK] Check package \"Bar\"")
        );
        assert_eq!(
            value.perfdata,
            Perfdata::Multiple(vec![
                String::from("'baz'=158;; "),
                String::from("'qux'=158;; ")
            ])
        );
    }

    #[test]
    fn test_deserialize_checker_empty_result() {
        let data = r#"{"exitcode":{},"checkresult":"","perfdata":{}}"#;
        let value: CheckerResult = serde_json::from_str(data).unwrap();
        assert_eq!(value.exitcode, Exitcode::NotExecuted(EmptyObject::new()));
        assert_eq!(value.checkresult, "");
        assert_eq!(value.perfdata, Perfdata::None(EmptyObject::new()));
    }
}
