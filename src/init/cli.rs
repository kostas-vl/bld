use crate::definitions::{AUTHOR, VERSION};
use clap::{App, SubCommand};

pub fn command() -> App<'static, 'static> {
    SubCommand::with_name("init")
        .about("Initializes the build configuration")
        .version(VERSION)
        .author(AUTHOR)
}
