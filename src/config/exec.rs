use crate::config::{BldConfig, BldLocalConfig, BldRemoteConfig};
use crate::helpers::term;
use crate::types::Result;
use clap::ArgMatches;

fn list_locals(local: &BldLocalConfig) -> Result<()> {
    term::print_info("Local configuration:")?;
    println!("- enable-server: {}", local.enable_server);
    println!("- host: {}", local.host);
    println!("- port: {}", local.port);
    println!("- logs: {}", local.logs);
    println!("- db: {}", local.db);
    println!("- docker-url: {}", local.docker_url);
    Ok(())
}

fn list_remote(remote: &BldRemoteConfig) -> Result<()> {
    term::print_info("Remote configuration:")?;

    for (i, server) in remote.servers.iter().enumerate() {
        println!("- name: {}", server.name);
        println!("- host: {}", server.host);
        println!("- port: {}", server.port);
        if i < remote.servers.len() - 1 {
            println!("");
        }
    }

    Ok(())
}

fn list_all(config: &BldConfig) -> Result<()> {
    list_locals(&config.local)?;
    println!("");
    list_remote(&config.remote)?;
    Ok(())
}

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    if matches.is_present("local") {
        return list_locals(&config.local);
    }
    if matches.is_present("remote") {
        return list_remote(&config.remote);
    }
    list_all(&config)
}
