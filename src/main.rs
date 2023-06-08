use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;
use twitch_irc::message::{ServerMessage, RGBColor};

use std::process::exit;
use ansi_term::Colour::RGB;

static HELP_MESSAGE: &str =
"tchat - Monitor any Twitch Chat on the Terminal
Usage: tchat <USERNAME>

`-h` or `--help` to Show this Message";

#[tokio::main]
async fn main() {
    // take the first arg as string
    let arg: &str = &std::env::args().skip(1).take(1).next().unwrap_or_else(|| {
        eprintln!("No Username Provided! try `tchat --help` for more info.");
        exit(2);
    });
        
    if arg == "-h" || arg == "--help" {
        eprintln!("{}", HELP_MESSAGE);
        exit(0);
    }

    // load the default config which will join anonymously
    let config = ClientConfig::default();
    let (mut message, client) = 
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    // handle the actual messages
    let join_handle = tokio::spawn(async move {
        while let Some(msg_recv) = message.recv().await {
            // filter chat messages only
            if let ServerMessage::Privmsg(msg) = msg_recv {
                // colour the username, white if none is set
                let colour = msg.name_color.unwrap_or(RGBColor {r: 255, g: 255, b: 255});
                let name_coloured = RGB(colour.r, colour.g, colour.b)
                    .bold()
                    .paint(msg.sender.name);
                
                // parse the badges to a vector
                let badges: Vec<String> = msg.badges
                    .iter()
                    .map(|badge| badge.name.to_string())
                    .collect();

                // FIXME this is a mess...
                // append different coloured strings based on which badge this is
                let mut badges_print: String = "".to_string();
                for badge in badges {
                    badges_print = match badge.as_ref() {
                        "broadcaster" => format!("{}{}", badges_print, &RGB(233, 25, 22).bold().paint("|cam|")),
                        "moderator" => format!("{}{}", badges_print, &RGB(00, 173, 03).bold().paint("|mod|")),
                        "vip" => format!("{}{}", badges_print, &RGB(224, 05, 185).bold().paint("|vip|")),
                        "subscriber" => format!("{}{}", badges_print, &RGB(130, 05, 180).bold().paint("|sub|")),
                        &_ => format!("{}|{}|", badges_print, badge),// do nothing if unknown
                    }
                }
                // add space to separate from username if there is at least one
                if !badges_print.is_empty() { badges_print.push_str(" "); }

                // print em all to terminal!
                println!("{}{}: {}", badges_print, name_coloured, msg.message_text);
            }
        }
    });

    // join that channel's twitch chat
    client.join(arg.to_owned()).unwrap_or_else(|_| {
        eprintln!("Invalid Username Provided!");
        exit(2);
    });
    
    // await messages
    join_handle.await.expect("Failed to Handle Message");
}
