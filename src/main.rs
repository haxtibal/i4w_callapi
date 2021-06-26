mod cli;
mod client;
mod icinga;
mod ps;
mod restapiv1;

use icinga::icinga_exit;

fn main() {
    let app = cli::Cli::parsed();
    let restapi_client = client::IcingaPsRestApiClient::new(&app.host, app.port, app.insecure);
    icinga_exit(restapi_client.checker_commnad(&app.command, &app.forward_args));
}
