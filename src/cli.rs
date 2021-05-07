use clap::{value_t, App, AppSettings, Arg};

fn parser<'a, 'b>() -> App<'a, 'b> {
    App::new("call_api_check")
        .about("Forward check plugin invocations to icinga-powershell-restapi daemon.")
        .version("0.1.0")
        .setting(AppSettings::TrailingVarArg)
        .arg(
            Arg::with_name("host")
                .long("host")
                .takes_value(true)
                .required(false)
                .help("Host where the daemon runs. Default: localhost."),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true)
                .required(false)
                .help("TCP port where the daemon listens. Default: 5668."),
        )
        .arg(
            Arg::with_name("command")
                .short("c")
                .long("command")
                .takes_value(true)
                .required(true)
                .help("Name or alias of the check plugin to execute. Example: Invoke-IcingaCheckCPU."),
        )
        .arg(
            Arg::with_name("insecure")
                .long("insecure")
                .takes_value(false)
                .required(false)
                .help("Ignore TLS certificate errors."),
        )
        .arg(
            Arg::with_name("ARGS")
                .takes_value(true)
                .multiple(true)
                .allow_hyphen_values(true)
                .help("Any number of options or paramters, forwarded to the check plugin. Positional arguments are ignored."),
        )
}

pub struct Cli {
    pub host: String,
    pub port: u32,
    pub command: String,
    pub insecure: bool,
    pub forward_args: Vec<String>,
}

impl Cli {
    pub fn parsed() -> Self {
        let app = parser();
        let mut cli = Cli {
            host: String::from("localhost"),
            port: 5668,
            command: String::new(),
            insecure: false,
            forward_args: Vec::new(),
        };
        let matches = app.get_matches();
        if let Ok(port) = value_t!(matches, "port", u32) {
            cli.port = port;
        }
        if let Some(command) = matches.value_of("command") {
            cli.command = String::from(command);
        }
        cli.insecure = matches.is_present("insecure");
        if let Some(forward_args) = matches.values_of("ARGS") {
            cli.forward_args = forward_args.map(|s| s.to_string()).collect();
        }
        cli
    }
}

#[test]
fn test_min_cli() {
    let matches = parser()
        .get_matches_from_safe(vec!["call_api_check", "--command", "Invoke-Foo", "--", "1"])
        .unwrap();
    assert_eq!(matches.value_of("command").unwrap(), "Invoke-Foo");
    assert_eq!(matches.is_present("insecure"), false);
    let trail: Vec<&str> = matches.values_of("ARGS").unwrap().collect();
    assert_eq!(trail, ["1"]);
}

#[test]
fn test_max_cli() {
    let matches = parser()
        .get_matches_from_safe(vec![
            "call_api_check",
            "--host",
            "localhost",
            "--port",
            "5668",
            "--command",
            "Invoke-Foo",
            "--insecure",
            "--",
            "-arg1",
            "1",
            "-arg2",
        ])
        .unwrap();
    assert_eq!(matches.value_of("host").unwrap(), "localhost");
    assert_eq!(value_t!(matches, "port", u32).unwrap(), 5668);
    assert_eq!(matches.value_of("command").unwrap(), "Invoke-Foo");
    assert_eq!(matches.is_present("insecure"), true);
    let trail: Vec<&str> = matches.values_of("ARGS").unwrap().collect();
    assert_eq!(trail, ["-arg1", "1", "-arg2"]);
}
