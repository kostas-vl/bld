use crate::auth::Login;
use crate::config::{definitions::VERSION, Auth, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::types::{BldCommand, Result};
use clap::{App, Arg, ArgMatches, SubCommand};

static LOGIN: &str = "login";
static SERVER: &str = "server";

pub struct AuthCommand;

impl AuthCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for AuthCommand {
    fn id(&self) -> &'static str {
        LOGIN
    }

    fn interface(&self) -> App<'static, 'static> {
        let server = Arg::with_name(SERVER)
            .short("s")
            .long("server")
            .help("The target bld server")
            .takes_value(true);
        SubCommand::with_name(LOGIN)
            .about("Initiates the login process for a bld server")
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
        if let Auth::OAuth2(info) = auth {
            info.login(name)?;
        }
        Ok(())
    }
}