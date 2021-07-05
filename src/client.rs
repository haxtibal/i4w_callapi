use crate::restapiv1;
use std::convert::TryFrom;
use std::time::Duration;

pub struct IcingaPsRestApiClient {
    host: String,
    port: u32,
    allow_invalid_certs: bool,
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

        let response = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(self.allow_invalid_certs)
            .connect_timeout(Duration::from_secs(60))
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap()
            .post(url)
            .json(&restapiv1::CommandArguments::try_from(args)?)
            .send()?;

        let body_data = response.json::<restapiv1::CheckerResponseBody>()?;

        body_data
            .into_iter()
            .next()
            .map(|(_key, value)| Ok(value))
            .unwrap_or_else(|| Err("No check result in API response.".into()))
    }
}
