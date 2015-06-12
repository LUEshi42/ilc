// Copyright 2015 Till Höppner
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![feature(libc, plugin)]
#![plugin(regex_macros)]

extern crate ilc;
extern crate chrono;
extern crate docopt;
extern crate rustc_serialize;
extern crate libc;
extern crate regex;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::fs::File;
use std::io::{ self, Read, BufRead, BufReader, Write };
use std::str::FromStr;

use docopt::Docopt;

use chrono::offset::fixed::FixedOffset;
use chrono::naive::date::NaiveDate;

use ilc::context::Context;
use ilc::event::Event;
use ilc::format::{ self, Encode, Decode };

static USAGE: &'static str = r#"
d8b   888
Y8P   888
      888
888   888    .d8888b
888   888   d88P"
888   888   888
888   888   Y88b.
888   888    "Y8888P

A converter and statistics utility for IRC log files.

Usage:
  ilc parse <file>...
  ilc convert <informat> <outformat> [--date DATE] [--tz SECS] [--channel CH]
  ilc (-h | --help | -v | --version)

Options:
  -h --help         Show this screen.
  -v --version      Show the version (duh).
  --date DATE       Override the date for this log. ISO 8601, YYYY-MM-DD.
  --tz SECONDS      UTC offset in the direction of the western hemisphere. [default: 0]
  --channel CH      Set a channel for the given log.
"#;

#[derive(RustcDecodable, Debug)]
struct Args {
    cmd_parse: bool,
    cmd_convert: bool,
    arg_file: Vec<String>,
    arg_informat: Option<String>,
    arg_outformat: Option<String>,
    flag_help: bool,
    flag_version: bool,
    flag_date: Option<String>,
    flag_tz: i32,
    flag_channel: Option<String>
}

/*fn encode<'a, W, F>(format: &str) -> F where F: Encode<'a, W> {
    match format {
        "weechat3" => format::weechat3::Weechat3,
        "energymech" => format::energymech::Energymech
    }
}*/

fn main() {
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
               .and_then(|d| d.decode())
               .unwrap_or_else(|e| e.exit());
    if args.flag_help {
        println!("{}", USAGE);
        unsafe { libc::funcs::c95::stdlib::exit(1) }
    }

    let context = Context {
        timezone: FixedOffset::west(args.flag_tz),
        override_date: args.flag_date.and_then(|d| NaiveDate::from_str(&d).ok()),
        channel: args.flag_channel.clone()
    };

    if args.cmd_parse {
        let mut parser = format::energymech::Energymech;
        let formatter = format::binary::Binary;
        for file in args.arg_file {
            let f: BufReader<File> = BufReader::new(File::open(file).unwrap());
            let iter = parser.decode(&context, f);
            for e in iter {
                info!("Parsed: {:?}", e);
                drop(formatter.encode(&context, io::stdout(), &e.unwrap()));
            }
        }
    }

    if args.cmd_convert {
        let stdin = io::stdin();

        let mut parser: &mut Decode<&mut BufRead, Box<Iterator<Item=Event>>> = match args.arg_informat.map(|s| s.as_ref()) {
            Some("energymech") => &mut format::energymech::Energymech,
            Some("weechat3") => &mut format::weechat3::Weechat3,
            Some("binary") => &mut format::binary::Binary
        };
        let formatter: &Encode<&mut Write> = match args.arg_outformat.map(|s| s.as_ref()) {
            Some("energymech") => &format::energymech::Energymech,
            Some("weechat3") => &format::weechat3::Weechat3,
            Some("binary") => &format::binary::Binary
        };

        for e in parser.decode(&context, &mut stdin.lock()) {
            drop(formatter.encode(&context, &mut io::stdout(), &e.unwrap()))
        }
    }
}
