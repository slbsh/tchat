use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;
use twitch_irc::message::{ServerMessage, RGBColor};
use twitch_irc::validate::validate_login;

use std::process::exit;
use ansi_term::Colour::RGB;
use tokio::sync::Mutex;
use lazy_static::lazy_static;
use async_trait::async_trait;

static HELP_MESSAGE: &str =
"tchat - Monitor any Twitch Chat on the Terminal
Usage: tchat [OPTION] <USERNAME>

Usage:
   -h, --help`    Show this Message
   -m, --mini     Use the minified badge display";

// initialize the minify flag
lazy_static! {
    static ref MINIFY_FLAG: Mutex<bool> = Mutex::new(false);
}

#[async_trait]
trait ColorStr {
    async fn colourize(&self, text: &str, alt_text: &str, r: u8, g: u8, b: u8) -> String;
}

// create a method for String for appending coloured text
#[async_trait]
impl ColorStr for String {
    async fn colourize(&self, text: &str, alt_text: &str, r: u8, g: u8, b: u8) -> String {
        // reassign the value as mutable
        let mut text: &str = text;

        // handle the minify flag
        let minify = MINIFY_FLAG.lock().await;
        if *minify { text = alt_text; }

        let mut res = format!("{}{}", &self, RGB(r, g, b).bold().paint(text));

        if *minify { res.insert_str(0, "|"); }

        res
    }
}

async fn parse_args() -> String {
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

    // check if we want to minify
    if arg.contains(&"-m".to_owned()) || arg.contains(&"--mini".to_owned()) {
        arg.retain(|s| s != "-m");
        arg.retain(|s| s != "-mini");
        let mut minify = MINIFY_FLAG.lock().await;
        *minify = true;
    }

    // parse user into &str
    let user: &str = &arg.into_iter().take(1).next().unwrap_or_else(|| {
        eprintln!("No Username Provided! try `tchat --help` for more info.");
        exit(2);
    });

    user.to_lowercase()
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

                // FIXME this is (still) a mess...
                // append different coloured strings based on which badge this is
                let mut badges_print: String = "".to_string();
                for badge in badges {
                    badges_print = match badge.as_ref() {
                        "broadcaster" => badges_print.colourize("|owner|","o", 233, 25, 22).await,
                        "moderator" => badges_print.colourize("|mod|","m", 00, 173, 03).await,
                        "vip" => badges_print.colourize("|vip|", "v", 224, 05, 185).await,
                        "subscriber" => badges_print.colourize("|sub|", "s", 130, 05, 180).await,
                        "founder" => badges_print.colourize("|1st|", "1s", 170, 64, 213).await,
                        "no_audio" => badges_print.colourize("|mute|", "/a", 50, 50, 57).await,
                        "no_video" => badges_print.colourize("|blind|", "/v", 50, 50, 57).await,
                        "game-developer" => badges_print.colourize("|dev|", "d", 50, 50, 57).await,
                        "bits" => badges_print.colourize("|bit|", "b", 193, 178, 17).await,
                        "prime_gaming" | "sub-gifter" | "premium" | "superultracombo-2023" 
                            | "bits-charity" => badges_print, //ignore
                        &_ => {
                            format!("{}|{}|", badges_print, badge) // no colour if unknown
                        },
                    }
                }
                // read the minify flag
                let minify = MINIFY_FLAG.lock().await;

                // add space to separate from username if there is at least one badge
                // and minify isn't on
                if !badges_print.is_empty() && !*minify { badges_print.push_str(" "); }
                // add a separator if it doesnt end with one
                if *minify && !badges_print.is_empty() { badges_print.push_str("|"); }

                // print em all to terminal!
                println!("{}{}: {}", badges_print, name_coloured, msg.message_text);
            }
        }
    });

    // check if the username is valid
    validate_login(&arg).unwrap_or_else(|_| {
        eprintln!("Invalid Username Provided!");
        exit(2);
    });

    // join that channel's twitch chat
    client.join(arg).expect("Failed to Join given Channel!");
    
    // await messages
    join_handle.await.expect("Failed to Handle Message");
}
