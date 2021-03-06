use crate::config::BldConfig;
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::headers;
use crate::helpers::term::print_error;
use crate::monit::MonitClient;
use crate::types::{MonitInfo, Result};
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use clap::ArgMatches;
use futures::stream::StreamExt;
use std::collections::HashMap;

struct MonitConnectionInfo {
    host: String,
    port: i64,
    headers: HashMap<String, String>,
    pip_id: Option<String>,
    pip_name: Option<String>,
    pip_last: bool,
}

async fn remote_invoke(info: MonitConnectionInfo) -> Result<()> {
    let url = format!("http://{}:{}/ws-monit", info.host, info.port);
    let mut client = Client::new().ws(url);
    for (key, value) in info.headers.iter() {
        client = client.header(&key[..], &value[..]);
    }
    let (_, framed) = client.connect().await?;
    let (sink, stream) = framed.split();
    let addr = MonitClient::create(|ctx| {
        MonitClient::add_stream(stream, ctx);
        MonitClient::new(SinkWrite::new(sink, ctx))
    });
    addr.send(MonitInfo::new(info.pip_id, info.pip_name, info.pip_last))
        .await?;
    Ok(())
}

fn exec_request(info: MonitConnectionInfo) {
    let system = System::new("bld-monit");
    Arbiter::spawn(async move {
        if let Err(e) = remote_invoke(info).await {
            let _ = print_error(&e.to_string());
            System::current().stop();
        }
    });
    let _ = system.run();
}

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let pip_id = matches.value_of("pipeline-id").map(|x| x.to_string());
    let pip_name = matches.value_of("pipeline").map(|x| x.to_string());
    let pip_last = matches.is_present("last");
    let srv = config.remote.server_or_first(matches.value_of("server"))?;
    let (name, auth) = match &srv.same_auth_as {
        Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid(),
        },
        None => (&srv.name, &srv.auth),
    };
    exec_request(MonitConnectionInfo {
        host: srv.host.to_string(),
        port: srv.port,
        headers: headers(name, auth)?,
        pip_id,
        pip_name,
        pip_last,
    });
    Ok(())
}
