use super::super::{
    msg::{Id, Message},
    str::OptName,
};
use super::{info, optbody, prelude::*, tok, tristatus};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("extra token: {0}")]
    ExtraToken(String),
    #[error(transparent)]
    UnexpectedEol(#[from] EolError),
    #[error("invalid best move, assuming null move")]
    InvalidBestmove(#[source] uci::RawParseError),
    #[error("invalid ponder move")]
    InvalidPonder(#[source] uci::RawParseError),
    #[error("invalid copy protection status")]
    InvalidCopyProtection(#[source] tristatus::Error),
    #[error("invalid registration status")]
    InvalidRegistration(#[source] tristatus::Error),
    #[error("cannot parse info #{}: {}", pos + 1, error)]
    BadInfo {
        pos: usize,
        #[source]
        error: info::Error,
    },
    #[error("no \"name\" in \"option\"")]
    OptionNoName,
    #[error("no \"type\" in \"setoption\"")]
    OptionNoType,
    #[error("cannot convert option name: {0}")]
    OptionBadName(#[source] StrError),
    #[error("invalid option body: {0}")]
    OptionBadBody(#[from] optbody::Error),
}

pub fn parse(tokens: &mut &[&UciToken], warn: &mut impl Sink<Error>) -> Option<Message> {
    let result = (|| loop {
        match tok::next(tokens)?.as_str() {
            "id" => {
                return match tok::next_warn(tokens, warn)?.as_str() {
                    "name" => {
                        let name = UciString::from_tokens(tokens);
                        *tokens = &[];
                        Some(Message::Id(Id::Name(name)))
                    }
                    "author" => {
                        let name = UciString::from_tokens(tokens);
                        *tokens = &[];
                        Some(Message::Id(Id::Author(name)))
                    }
                    tok => {
                        warn.warn(Error::UnexpectedToken(tok.to_string()));
                        None
                    }
                }
            }
            "uciok" => return Some(Message::UciOk),
            "readyok" => return Some(Message::ReadyOk),
            "bestmove" => {
                let bestmove = tok::next_warn(tokens, warn)?;
                let bestmove = bestmove
                    .parse()
                    .or_warn_map(Error::InvalidBestmove, warn)
                    .unwrap_or(UciMove::Null);
                let ponder = (|| {
                    let tok = tok::next(tokens)?;
                    if tok != "ponder" {
                        warn.warn(Error::UnexpectedToken(tok.to_string()));
                        return None;
                    }
                    let ponder = tok::next_warn(tokens, warn)?;
                    ponder.parse().or_warn_map(Error::InvalidPonder, warn).ok()
                })();
                return Some(Message::BestMove { bestmove, ponder });
            }
            "copyprotection" => {
                let status =
                    tristatus::parse(tokens, &mut warn.adapt_map(Error::InvalidCopyProtection))?;
                return Some(Message::CopyProtection(status));
            }
            "registration" => {
                let status =
                    tristatus::parse(tokens, &mut warn.adapt_map(Error::InvalidRegistration))?;
                return Some(Message::Registration(status));
            }
            "info" => {
                let mut info = Vec::new();
                let mut string = None;
                while !tokens.is_empty() {
                    if tokens[0] == "string" {
                        string = Some(UciString::from_tokens(&tokens[1..]));
                        *tokens = &[];
                        break;
                    }
                    let pos = info.len();
                    if let Some(inf) = info::parse(
                        tokens,
                        &mut warn.adapt_map(|error| Error::BadInfo { pos, error }),
                    ) {
                        info.push(inf);
                    }
                }
                return Some(Message::Info { info, string });
            }
            "option" => {
                tok::expect(tokens, "name", Error::OptionNoName, warn)?;
                let (name, mut body) = tok::split(tokens, "type", Error::OptionNoType, warn);
                *tokens = &[];
                let name = OptName::from_tokens(name)
                    .or_warn_map(Error::OptionBadName, warn)
                    .ok()?;
                let body = optbody::parse(&mut body, &mut warn.adapt())?;
                return Some(Message::Option { name, body });
            }
            tok => warn.warn(Error::UnexpectedToken(tok.to_string())),
        }
    })();
    if !tokens.is_empty() {
        warn.warn(Error::UnexpectedToken(tokens[0].to_string()));
    }
    result
}
