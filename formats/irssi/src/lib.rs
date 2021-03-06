use std::io::{BufRead, Write};
use std::borrow::ToOwned;
use std::iter::Iterator;

use event::{Event, Time, Type};
use context::Context;
use format::{Decode, Encode, rejoin, strip_one};

use l::LogLevel::Info;

pub struct Irssi;

static LOG_OPEN_FORMAT: &'static str = "%a %b %e %T %Y";
static LINE_FORMAT: &'static str = "%H:%M";

pub struct Iter<'a> {
    context: &'a Context,
    input: &'a mut BufRead,
    buffer: Vec<u8>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = ::Result<Event<'a>>;
    fn next(&mut self) -> Option<::Result<Event<'a>>> {
        fn parse_time(c: &Context, date: &str, time: &str) -> Time {
            Time::from_format(&c.timezone, &format!("{} {}", date, time), TIME_DATE_FORMAT)
        }

        loop {
            self.buffer.clear();
            match self.input.read_until(b'\n', &mut self.buffer) {
                Ok(0) | Err(_) => return None,
                Ok(_) => (),
            }

            let buffer = String::from_utf8_lossy(&self.buffer);

            let mut split_tokens: Vec<char> = Vec::new();
            let tokens = buffer.split(|c: char| {
                                   if c.is_whitespace() {
                                       split_tokens.push(c);
                                       true
                                   } else {
                                       false
                                   }
                               })
                               .collect::<Vec<_>>();

            if log_enabled!(Info) {
                info!("Original:  `{}`", buffer);
                info!("Parsing:   {:?}", tokens);
            }

            match &tokens[..tokens.len() - 1] {
                ["---", "Log", "opened", day_of_week, month, day, time, year] => year,
                ["---", "Log", "closed", day_of_week, month, day, time, year] => {
                    return Some(Ok(Event {
                        ty: Type::Disconnect,
                        time: parse_time(&self.context, date, time),
                        channel: self.context.channel.clone().map(Into::into),
                    }))
                }
                [time, "-!-", nick, host, "has", "joined", channel] => {
                    return Some(Ok(Event {
                        ty: Type::Join {
                            nick: nick.to_owned().into(),
                            mask: Some(strip_one(host).into()),
                        },
                        channel: Some(channel.to_owned().into()),
                        time: parse_time(&self.context, date, time),
                    }))
                }
                [time, "-!-", nick, host, "has", "left", channel, reason..] => {
                    return Some(Ok(Event {
                        ty: Type::Part {
                            nick: nick.to_owned().into(),
                            mask: Some(strip_one(host).into()),
                            reason: Some(strip_one(&rejoin(reason, &split_tokens[8..])).into()),
                        },
                        channel: Some(channel.to_owned().into()),
                        time: parse_time(&self.context, date, time),
                    }))
                }
                [time, "-!-", nick, host, "has", "quit", reason..] => {
                    return Some(Ok(Event {
                        ty: Type::Quit {
                            nick: nick.to_owned().into(),
                            mask: Some(strip_one(host).into()),
                            reason: Some(strip_one(&rejoin(reason, &split_tokens[7..])).into()),
                        },
                        time: parse_time(&self.context, date, time),
                        channel: self.context.channel.clone().map(Into::into),
                    }))
                }
                // TODO: reorder
                [date, time, "--", notice, content..] if notice.starts_with("Notice(") => {
                    return Some(Ok(Event {
                        ty: Type::Notice {
                            from: notice["Notice(".len()..notice.len() - 2].to_owned().into(),
                            content: rejoin(content, &split_tokens[4..]),
                        },
                        time: parse_time(&self.context, date, time),
                        channel: self.context.channel.clone().map(Into::into),
                    }))
                }
                [date, time, "--", nick, verb, "now", "known", "as", new_nick] if verb == "is" ||
                                                                                  verb == "are" => {
                    return Some(Ok(Event {
                        ty: Type::Nick {
                            old_nick: nick.to_owned().into(),
                            new_nick: new_nick.to_owned().into(),
                        },
                        time: parse_time(&self.context, date, time),
                        channel: self.context.channel.clone().map(Into::into),
                    }))
                }
                [date, time, sp, "*", nick, msg..] if sp.clone().is_empty() => {
                    return Some(Ok(Event {
                        ty: Type::Action {
                            from: nick.to_owned().into(),
                            content: rejoin(msg, &split_tokens[5..]),
                        },
                        time: parse_time(&self.context, date, time),
                        channel: self.context.channel.clone().map(Into::into),
                    }))
                }
                [date, time, nick, msg..] => {
                    return Some(Ok(Event {
                        ty: Type::Msg {
                            from: nick.to_owned().into(),
                            content: rejoin(msg, &split_tokens[3..]),
                        },
                        time: parse_time(&self.context, date, time),
                        channel: self.context.channel.clone().map(Into::into),
                    }))
                }
                _ => (),
            }
        }
    }
}

impl Decode for Irssi {
    fn decode<'a>(&'a mut self,
                  context: &'a Context,
                  input: &'a mut BufRead)
                  -> Box<Iterator<Item = ::Result<Event<'a>>> + 'a> {
        Box::new(Iter {
            context: context,
            input: input,
            buffer: Vec::new(),
        })
    }
}

impl Encode for Irssi {
    fn encode<'a>(&'a self,
                  context: &'a Context,
                  mut output: &'a mut Write,
                  event: &'a Event)
                  -> ::Result<()> {
        match event {
            &Event { ty: Type::Msg { ref from, ref content, .. }, ref time, .. } => {
                try!(writeln!(&mut output,
                              "{}\t{}\t{}",
                              time.with_format(&context.timezone, TIME_DATE_FORMAT),
                              from,
                              content))
            }
            &Event { ty: Type::Action { ref from, ref content, .. }, ref time, .. } => {
                try!(writeln!(&mut output,
                              "{}\t *\t{} {}",
                              time.with_format(&context.timezone, TIME_DATE_FORMAT),
                              from,
                              content))
            }
            &Event { ty: Type::Join { ref nick, ref mask, .. }, ref channel, ref time } => {
                try!(writeln!(&mut output,
                              "{}\t-->\t{} ({}) has joined {}",
                              time.with_format(&context.timezone, TIME_DATE_FORMAT),
                              nick,
                              mask.as_ref().expect("Hostmask not present, but required."),
                              channel.as_ref().expect("Channel not present, but required.")))
            }
            &Event { ty: Type::Part { ref nick, ref mask, ref reason }, ref channel, ref time } => {
                try!(write!(&mut output,
                            "{}\t<--\t{} ({}) has left {}",
                            time.with_format(&context.timezone, TIME_DATE_FORMAT),
                            nick,
                            mask.as_ref().expect("Hostmask not present, but required."),
                            channel.as_ref().expect("Channel not present, but required.")));
                if reason.is_some() && reason.as_ref().unwrap().len() > 0 {
                    try!(write!(&mut output, " ({})", reason.as_ref().unwrap()));
                }
                try!(write!(&mut output, "\n"))
            }
            &Event { ty: Type::Quit { ref nick, ref mask, ref reason }, ref time, .. } => {
                try!(write!(&mut output,
                            "{}\t<--\t{} ({}) has quit",
                            time.with_format(&context.timezone, TIME_DATE_FORMAT),
                            nick,
                            mask.as_ref().expect("Hostmask not present, but required.")));
                if reason.is_some() && reason.as_ref().unwrap().len() > 0 {
                    try!(write!(&mut output, " ({})", reason.as_ref().unwrap()));
                }
                try!(write!(&mut output, "\n"))
            }
            &Event { ty: Type::Disconnect, ref time, .. } => {
                try!(writeln!(&mut output,
                              "{}\t--\tirc: disconnected from server",
                              time.with_format(&context.timezone, TIME_DATE_FORMAT)))
            }
            &Event { ty: Type::Notice { ref from, ref content }, ref time, .. } => {
                try!(writeln!(&mut output,
                              "{}\t--\tNotice({}): {}",
                              time.with_format(&context.timezone, TIME_DATE_FORMAT),
                              from,
                              content))
            }
            _ => (),
        }
        Ok(())
    }
}
