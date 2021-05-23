use crate::config::{definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::{exec_get, headers};
use crate::types::{BldCommand, Result};
use clap::{App, Arg, ArgMatches, SubCommand};

static HIST: &str = "hist";
static SERVER: &str = "server";

pub struct HistCommand;

impl HistCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(HistCommand)
    }
}

impl BldCommand for HistCommand {
    fn id(&self) -> &'static str {
        HIST
    }

    fn interface(&self) -> App<'static, 'static> {
        let server = Arg::with_name(SERVER)
            .short("s")
            .long("server")
            .takes_value(true)
            .help("The name of the server from which to fetch execution history");
        SubCommand::with_name(HIST)
            .about("Fetches execution history of pipelines on a server")
            .version(VERSION)
            .args(&[server])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> Result<()> {
        let config = BldConfig::load()?;
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        let sys = String::from("bld-hist");
        let url = format!("http://{}:{}/hist", srv.host, srv.port);
        let headers = headers(name, auth)?;
        exec_get(sys, url, headers);
        Ok(())
    }
}