use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;
use twitch_irc::message::{ServerMessage, RGBColor};

use std::process::exit;
use ansi_term::Colour::RGB;

static HELP_MESSAGE: &str =
"tchat - Monitor any Twitch Chat on the Terminal
Usage: tchat [OPTION] <USERNAME>

Usage:
   -h, --help`    Show this Message
   -m, --mini     Use the minified badge display";

trait ColorStr {
    fn colourize(&self, mini: bool, text: &str, alt_text: &str, r: u8, g: u8, b: u8) -> String;
}

// create a method for String for appending coloured text
impl ColorStr for String {
    fn colourize(&self, mini: bool, text: &str, alt_text: &str, r: u8, g: u8, b: u8) -> String {
        let res: String;

        // handle the minify flag, choosing the appropriate version
        if mini { 
            res = format!("{}{}", &self, RGB(r, g, b).bold().paint(alt_text));
        } else {
            res = format!("{}{}", &self, RGB(r, g, b).bold().paint(text));
        }

        res
    }
}

async fn parse_args() -> (String, bool) {
    let mut arg: Vec<String> = std::env::args().skip(1).collect();

    // return if no args are given
    if arg.len() == 0 {
        eprintln!("No Username Provided! try `tchat --help` for more info.");
        exit(2);
    }

    // handle informational args
    if &arg[0] == "-h" || &arg[0] == "--help" {
        eprintln!("{}", HELP_MESSAGE);
        exit(0);
    }
    if &arg[0] == "-v" || &arg[0] == "--version" {
        eprintln!("0.2.0");
        exit(0);
    }
    
    let mut minify = false;

    // check if we want to minify
    if arg.contains(&"-m".to_owned()) || arg.contains(&"--mini".to_owned()) {
        // remove the arg given that we already parsed it
        arg.retain(|s| s != "-m" || s != "--mini");
        // set the flag
        minify = true;
    }

    // parse user into &str
    let user: &str = &arg.into_iter().take(1).next().unwrap_or_else(|| {
        eprintln!("No Username Provided! try `tchat --help` for more info.");
        exit(2);
    });

    (user.to_lowercase(), minify)
}

#[tokio::main]
async fn main() {
    // checks args and returns username and minify bool
    let arg = parse_args().await; 

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
                    .paint(&msg.sender.name);
                
                // parse the badges to a vector
                let badges: Vec<String> = msg.badges
                    .iter()
                    .map(|badge| badge.name.to_string())
                    .collect();

                // append different coloured strings based on which badge this is
                let mut badges_print: String = "".to_string();
                for badge in badges {
                    badges_print = match badge.as_ref() {
                        "broadcaster" => badges_print.colourize(arg.1, "|ttv|","t", 233, 25, 22),
                        "moderator" => badges_print.colourize(arg.1, "|mod|","m", 00, 173, 03),
                        "vip" => badges_print.colourize(arg.1, "|vip|", "v", 224, 05, 185),
                        "subscriber" => badges_print.colourize(arg.1, "|sub|", "s", 130, 05, 180),
                        "founder" => badges_print.colourize(arg.1, "|1st|", "s", 170, 64, 213),
                        "bits" => badges_print.colourize(arg.1, "|bit|", "b", 193, 178, 17),
                        &_ => badges_print, // no colour if unknown
                    }
                }
                // add space to separate from username if there is at least one badge
                // and minify isn't on
                if !badges_print.is_empty() && arg.1 { badges_print.push(' '); }

                // if minify is on then use a pipe instead of a space
                if !badges_print.is_empty() && arg.1 { badges_print.push('|'); }

                // print em all to terminal!
                println!("{}{}: {}", badges_print, name_coloured, msg.message_text);
            }
        }
    });

    // join that channel's twitch chat
    client.join(arg.0).unwrap_or_else(|_| {
        eprintln!("Invalid Username Provided!");
        exit(2);
    });
    
    // await messages
    join_handle.await.expect("Failed to Handle Message");
}
