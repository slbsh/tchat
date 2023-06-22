use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;
use twitch_irc::message::ServerMessage;

use std::process::exit;
use ansi_term::Colour::RGB;

static HELP_MESSAGE: &str =
"tchat - Monitor any Twitch Chat on the Terminal
Usage: tchat [OPTION] <USERNAME>

Usage:
   -h, --help`    Show this Message
   -m, --mini     Use the minified badge display";

async fn parse_args() -> String {
    let arg: &str = &std::env::args().skip(1).take(1).next().unwrap_or_else(|| {
        eprintln!("No Arguments Provided! try `tchat --help` for more info.");
        exit(2);
    });

    // handle informational args
    match arg {
        "-h" | "--help" => {
            eprintln!("{}", HELP_MESSAGE);
            exit(0);
        },
        "-v" | "--version" => {
            eprintln!("0.2.0");
            exit(0);
        },
        &_ => (),
    }
    
    // return arg as username
    arg.to_lowercase()
}

#[tokio::main]
async fn main() {
    // checks args and returns username and minify bool
    let username = parse_args().await; 

    // load the default config which will join anonymously
    let config = ClientConfig::default();
    let (mut message, client) = 
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    // handle the actual messages
    let join_handle = tokio::spawn(async move {
        while let Some(msg_recv) = message.recv().await {
            // filter chat messages only
            if let ServerMessage::Privmsg(msg) = msg_recv {
                // parse the badges to a vector
                let badges: String = msg.badges
                    .iter()
                    .take(1)
                    .map(|badge| badge.name.to_string())
                    .next()
                    .unwrap_or("".to_string());

                let colour = match badges.as_str() {
                    "broadcaster" => RGB(233, 25, 22 ),
                    "moderator"   => RGB(00, 173, 03 ),
                    "vip"         => RGB(244, 05, 185),  
                    "founder"     => RGB(170, 64, 213),  
                    "subscriber"  => RGB(130, 05, 180),  
                    "bits"        => RGB(193, 178, 17),
                    &_ => RGB(255, 255, 255),
                };

                // colour the username
                let name_coloured = colour.bold().paint(&msg.sender.name);

                // print em all to terminal!
                println!("{}: {}", name_coloured, msg.message_text);
            }
        }
    });

    // join that channel's twitch chat
    client.join(username).unwrap_or_else(|_| {
        eprintln!("Invalid Username Provided!");
        exit(2);
    });
    
    // await messages
    if let Err(why) = join_handle.await {
        eprintln!("msg handler Err: {}", why);
    }
}
