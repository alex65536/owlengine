use super::prelude::*;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("cannot parse move #{}: {}", pos + 1, error)]
pub struct Error {
    pub pos: usize,
    #[source]
    pub error: uci::RawParseError,
}

fn looks_like_move(tok: &UciToken) -> bool {
    let bytes = tok.as_bytes();
    matches!(bytes.len(), 4 | 5)
        && bytes[0].is_ascii_lowercase()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_lowercase()
        && bytes[3].is_ascii_digit()
}

pub fn parse(
    tokens: &mut &[&UciToken],
    until_first_error: bool,
    warn: &mut impl Sink<Error>,
) -> Vec<UciMove> {
    let mut moves = Vec::new();
    while !tokens.is_empty() {
        let tok = tokens[0];
        if !looks_like_move(tok) {
            break;
        }
        *tokens = &tokens[1..];
        match tok.parse::<UciMove>() {
            Ok(mv) => moves.push(mv),
            Err(error) => {
                warn.warn(Error {
                    pos: moves.len(),
                    error,
                });
                if until_first_error {
                    break;
                }
            }
        }
    }
    moves
}

pub fn fmt(src: &[UciMove], f: &mut impl PushTokens) {
    for mv in src {
        f.do_tok(&mv.to_string());
    }
}
