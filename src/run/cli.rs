use clap::{App, Arg, SubCommand};
use crate::definitions::{VERSION, AUTHOR};

pub fn command() -> App<'static, 'static> {
    let pipeline = Arg::with_name("pipeline")
        .short("p")
        .long("pipeline")
        .help("path to pipeline script")
        .takes_value(true);

    SubCommand::with_name("run")
        .about("Executes a build pipeline")
        .version(VERSION)
        .author(AUTHOR)
        .arg(pipeline)
}