use std::{env, fmt::Debug, io, process};

use wurm::Stderr;

use owlengine::uci::{
    msg::{Command, Message},
    parse::{Fmt, Parse},
};

fn do_uci_explore<P>()
where
    P: Parse + Fmt + Debug,
{
    let mut warn = Stderr;
    for line in io::stdin().lines() {
        let item = P::parse_line(&line.unwrap(), &mut warn);
        if let Some(item) = item {
            println!("{}", item.fmt_line());
            eprintln!("{:?}", item);
        } else {
            println!("<none>");
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 || !matches!(args[1].as_str(), "cmd" | "msg") {
        eprintln!("usage: uciecho cmd|msg");
        process::exit(1);
    }
    match args[1].as_str() {
        "cmd" => do_uci_explore::<Command>(),
        "msg" => do_uci_explore::<Message>(),
        arg => panic!("unknown arg {}", arg),
    }
}
