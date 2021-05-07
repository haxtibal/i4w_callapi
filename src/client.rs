use crate::restapiv1;
use indexmap::IndexMap;
use std::time::Duration;

pub struct IcingaPsRestApiClient {
    host: String,
    port: u32,
    allow_invalid_certs: bool,
}

fn build_command_map(args: &[String]) -> restapiv1::CommandArguments {
    let mut command_map: IndexMap<String, restapiv1::Argument> = IndexMap::new();
    let mut arg_value;

    for idx in 0..args.len() {
        if args[idx].starts_with('-') {
            if idx + 1 < args.len() {
                let next_value = &args[idx + 1];
                if next_value.starts_with('-') {
                    arg_value = restapiv1::Argument::DummyArgument(true);
                } else {
                    arg_value = restapiv1::Argument::RealArgument(next_value.clone());
                }
            } else {
                arg_value = restapiv1::Argument::DummyArgument(true);
            }
            command_map.insert(args[idx].replace("-", ""), arg_value);
        } else {
            continue;
        }
    }
    command_map
}

impl IcingaPsRestApiClient {
    pub fn new(host: &str, port: u32, allow_invalid_certs: bool) -> Self {
        IcingaPsRestApiClient {
            host: String::from(host),
            port,
            allow_invalid_certs,
        }
    }

    pub fn checker_commnad(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<restapiv1::CheckerResult, Box<dyn std::error::Error>> {
        let url = format!(
            "https://{}:{}/v1/checker?command={}",
            self.host, self.port, command
        );
        let command_map = build_command_map(args);

        let response = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(self.allow_invalid_certs)
            .connect_timeout(Duration::from_secs(60))
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap()
            .post(url)
            .json(&command_map)
            .send()?;

        let body_data = response.json::<restapiv1::CheckerResponseBody>()?;

        body_data
            .into_iter()
            .next()
            .map(|(_key, value)| Ok(value))
            .unwrap_or_else(|| Err("No check result in API response.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::build_command_map;
    use crate::restapiv1;

    #[test]
    fn test_build_command_map() {
        // positional arguments are not supported
        let args = vec![
            String::from("foo"),
            String::from("bar"),
            String::from("baz"),
        ];
        let mymap = build_command_map(args.as_slice());
        assert_eq!(mymap.len(), 0);

        // parameters with arguments are inserted as key value pairs
        let args = vec![
            String::from("-Warning"),
            String::from("0"),
            String::from("-Critical"),
            String::from("1"),
        ];
        let mymap = build_command_map(args.as_slice());
        assert_eq!(mymap.len(), 2);
        assert_eq!(
            mymap.get("Warning").unwrap(),
            &restapiv1::Argument::RealArgument(String::from("0"))
        );
        assert_eq!(
            mymap.get("Critical").unwrap(),
            &restapiv1::Argument::RealArgument(String::from("1"))
        );

        // switch arguments can be interleaved anywhere, fake value True is inserted
        let args = vec![
            String::from("-Warning"),
            String::from("0"),
            String::from("-switch"),
            String::from("-Critical"),
            String::from("1"),
        ];
        let mymap = build_command_map(args.as_slice());
        assert_eq!(mymap.len(), 3);
        assert_eq!(
            mymap.get("Warning").unwrap(),
            &restapiv1::Argument::RealArgument(String::from("0"))
        );
        assert_eq!(
            mymap.get("Critical").unwrap(),
            &restapiv1::Argument::RealArgument(String::from("1"))
        );
        assert_eq!(
            mymap.get("switch").unwrap(),
            &restapiv1::Argument::DummyArgument(true)
        );
    }
}
