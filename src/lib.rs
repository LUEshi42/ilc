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

#![feature(slice_patterns)]
extern crate chrono;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate bincode;
extern crate rmp;
extern crate rmp_serialize as msgpack;

pub mod event;
pub mod format;
pub mod context;

use std::convert::From;
use std::{io, result};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use chrono::format::ParseError;

pub type Result<T> = result::Result<T, IlcError>;

#[derive(Debug)]
pub enum IlcError {
    Parse(String),
    Chrono(ParseError),
    BincodeDecode,
    BincodeEncode,
    MsgpackEncode(msgpack::encode::Error),
    MsgpackDecode(msgpack::decode::Error),
    Io(io::Error),
}

impl Display for IlcError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_str(self.description())
    }
}

impl Error for IlcError {
    fn description(&self) -> &str {
        use IlcError::*;
        match self {
            &Parse(_) => "error while parsing",
            &Chrono(_) => "error while parsing time strings",
            &BincodeDecode => "error while decoding from binary",
            &BincodeEncode => "error while encoding to binary",
            &MsgpackDecode(_) => "error while decoding from msgpack",
            &MsgpackEncode(_) => "error while encoding to msgpack",
            &Io(_) => "error during input/output",
        }
    }

    fn cause(&self) -> Option<&Error> {
        use IlcError::*;
        match self {
            &Parse(ref _e) => None,
            &Chrono(ref e) => Some(e),
            &BincodeDecode => None,
            &BincodeEncode => None,
            &MsgpackDecode(ref e) => Some(e),
            &MsgpackEncode(ref e) => Some(e),
            &Io(ref e) => Some(e),
        }
    }
}

impl From<ParseError> for IlcError {
    fn from(err: ParseError) -> IlcError {
        IlcError::Chrono(err)
    }
}

impl From<io::Error> for IlcError {
    fn from(err: io::Error) -> IlcError {
        IlcError::Io(err)
    }
}
