use super::super::{
    msg::{Id, Message},
    str::OptName,
};
use super::{info, optbody, prelude::*, tok, tristatus};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
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

pub fn parse(tokens: &mut &[&Token], warn: &mut impl Warn<Error>) -> Option<Message> {
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
                    tok::parse_map(tokens, Error::InvalidPonder, warn)
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
                    .or_warn_map(Error::OptionBadName, warn)?;
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

pub fn fmt(src: &Message, f: &mut impl PushTokens) {
    match src {
        Message::Id(id) => {
            f.push_kw("id");
            match id {
                Id::Name(name) => f.push_tag_many("name", name),
                Id::Author(author) => f.push_tag_many("author", author),
            }
        }
        Message::UciOk => f.push_kw("uciok"),
        Message::ReadyOk => f.push_kw("readyok"),
        Message::BestMove { bestmove, ponder } => {
            f.push_tag("bestmove", bestmove);
            if let Some(ponder) = ponder {
                f.push_tag("ponder", ponder);
            }
        }
        Message::CopyProtection(status) => {
            f.push_kw("copyprotection");
            tristatus::fmt(status, f);
        }
        Message::Registration(status) => {
            f.push_kw("registration");
            tristatus::fmt(status, f);
        }
        Message::Info { info, string } => {
            f.push_kw("info");
            for inf in info {
                info::fmt(inf, f);
            }
            if let Some(string) = string {
                f.push_tag_many("string", string);
            }
        }
        Message::Option { name, body } => {
            f.push_kw("option");
            f.push_tag_many("name", name);
            f.push_kw("type");
            optbody::fmt(body, f);
        }
    }
}
