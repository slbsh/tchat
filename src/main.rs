use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;
use twitch_irc::message::ServerMessage;

use std::process::exit;
use ansi_term::Colour::{RGB, self};

#[derive(Default, Debug)]
struct Args {
    colour:  bool,
    bcolour: bool,
    badge:   bool,
    mbadge:  bool,
    fbadge:  bool,
    bits:    bool,
    origin:  bool,
    time:    bool,
    file:    Option<&'static str>,
    debug:   bool,
    quiet:   bool,
    names:   Vec<String>,
    ignore:  Vec<&'static str>,
}

const USAGE: &str = "Usage: tchat [-bBcCdFhoqtTV] [-f file] [username ...]";
const ERR_MESSAGE: &str = "Try the `-h` flag for more info.";
const HELP_MESSAGE: &str =
"\x1b[1mDESCRIPTION\x1b[0m
    \x1b[4mtchat\x1b[0m logs a twitch chat into the terminal, allowing for the monitoring during streams.
    Whenever multiple usernames are specified both of the chats will be monitored simultaneously.

    All of the colour options work properly only within \x1b[4mtruecolor\x1b[0m terminals.
    
\x1b[1mOPTIONS\x1b[0m
    -V  print current program version
    -h  print this message

    -b  print badges, shortened to 3 letters
    -B  print badges, shortened to a single letter. Printing `?` for non-common ones
    -F  print full badge names

    -c  colour the names based on twitch assigned colours
    -C  colour the names based on the first badge

    -t  show bits donations in amount
    -T  prepend the time of each message as HH:MM.
    -o  display the username of the channel the message came from

    -d  print debug info, propably not needed for mere mortals
    -q  dont print messages to stdout

    -f FILE 
        log the output to FILE with ansi escape codes striped. 
        This will append if the FILE already exists, and create one if not.

    -i USERNAME
        dont display chat messages coming from USERNAME.
        This option may be repeated to filter multiple users.
";

fn parse_args(args: Vec<String>) -> Args {
    let mut out = Args::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        if let Some(arg) = arg.strip_prefix('-') {
            for c in arg.chars() {
                match c {
                    'h' => {
                        println!("{}\n\n{}", USAGE, HELP_MESSAGE);
                        exit(0);
                    },

                    'V' => {
                        println!("{}", env!("CARGO_PKG_VERSION"));
                        exit(0);
                    },

                    /* colour usernames based on twitch assigned colours */
                    'c' => out.colour = true,
                    /* colour the usernames based on badges*/
                    'C' => out.bcolour = true,

                    /* display badges */
                    'b' => out.badge = true,
                    /* display badges with the minified display */
                    'B' => out.mbadge = true,
                    /* display the full badge names */
                    'F' => out.fbadge = true,

                    /* show bits donations*/
                    't' => out.bits = true,

                    /* display the chat that the message was sent in */
                    'o' => out.origin = true,

                    /* display the current time of the message */
                    'T' => out.time = true,

                    /* log to file */
                    'f' => out.file = match args.next() {
                        Some(a) => Some(Box::leak(a.clone().into_boxed_str())),
                        None => {
                            println!("Missing argument after `-f`\n {}\n{}", USAGE, ERR_MESSAGE);
                            exit(1);
                        },
                    },

                    /* ignore certain usernames (this can be repeated so we use a Vec<>) */ 
                    'i' => out.ignore.push(match args.next() {
                        Some(a) => Box::leak(a.clone().into_boxed_str()),
                        None => {
                            println!("Missing argument after `-i`\n {}\n{}", USAGE, ERR_MESSAGE);
                            exit(1);
                        },
                    }),

                    /* debug log */
                    'd' => out.debug = true,

                    /* dont print stdout */
                    'q' => out.quiet = true,

                    _ => {
                        println!("{}\n{}", USAGE, ERR_MESSAGE);
                        exit(1);
                    },
                }
            } continue;
        } /* not starting with `-` -> */ out.names.push(arg); 
    } out
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        println!("{}\n{}", USAGE, ERR_MESSAGE);
        exit(1);
    }

    // parse into Args
    let args = parse_args(args);
    if args.debug { dbg!(&args); }

    // load the default config which will join anonymously
    let config = twitch_irc::ClientConfig::default();
    let (mut message, client) = 
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    // handle the actual messages
    let join_handle = tokio::spawn(async move {
        while let Some(msg_recv) = message.recv().await {
            // filter chat messages only
            if let ServerMessage::Privmsg(msg) = msg_recv {
                if args.debug { dbg!(&msg); }

                if args.ignore.contains(&msg.sender.name.as_str()) {
                    continue;
                }

                // parse the badges to a vector
                let badges: Vec<String> = msg.badges
                    .iter()
                    .map(|b| b.name.to_string())
                    .collect();

                let name_colour: Option<Colour> = {
                    if args.colour { Some(_colour(msg.name_color)) }
                    else if args.bcolour { Some(_bcolour(&badges)) }
                    else { None }
                };

                let display_badges: Vec<(String, Colour)> = {
                    if args.badge { _badge(&badges) }
                    else if args.mbadge { _mbadge(&badges) }
                    else if args.fbadge { _fbadge(&badges) }
                    else { Vec::new() }
                };



                let mut message = String::new();
                /* add current time */
                if args.time {
                    message.push_str(&chrono::Local::now().format("%H:%M ").to_string());
                }

                /* add origin */
                if args.origin {
                    message.push_str(&format!("[{}]: ", msg.channel_login));
                }

                /* add bits */
                if args.bits && msg.bits.is_some() {
                    message.push_str(&BITS_COLOUR.bold().paint(format!("!{}!", msg.bits.unwrap())).to_string());
                }

                /* add badges */
                if !display_badges.is_empty() {
                    display_badges.iter()
                        .for_each(|(b, c)| {
                            message.push_str(&c.bold().paint(format!("|{}|", b)).to_string());
                        });
                }

                // add separator + little hack to make time display work
                if !message.is_empty() && message.chars().last().unwrap() != ' ' {
                    message.push(' '); 
                }

                /* add name */
                if let Some(colour) = name_colour {
                    message.push_str(&colour.bold().paint(format!("{}: ", msg.sender.name)).to_string());
                } else {
                    message.push_str(&format!("{}: ", msg.sender.name));
                }

                message.push_str(&msg.message_text);
                message.push('\n'); //needed for saving to file

                if !args.quiet {
                    // print em all!
                    print!("{}", message);
                }

                // maybe log to file too
                if let Some(file) = args.file {
                    let mut fd = std::fs::OpenOptions::new()
                        .write(true).append(true).create(true)
                        .open(file).unwrap_or_else(|e| {
                            eprintln!("{e}");
                            exit(1);
                        });

                    let plain_bytes = strip_ansi_escapes::strip(message.as_bytes());

                    use std::io::Write;
                    fd.write_all(&plain_bytes)
                        .unwrap_or_else(|e| {
                        eprintln!("{e}");
                        exit(1);
                    });
                }
            }
        }
    });

    // join that channel's twitch chat
    for name in args.names.into_iter() {
        client.join(name).unwrap_or_else(|n| {
            eprintln!("Failed to join: {n}");
            exit(1);
        });
    }
    
    // await messages
    if let Err(why) = join_handle.await {
        eprintln!("msg handler Err: {}", why);
    }
}

const BROADCASTER_COLOUR: Colour = RGB(233, 25,  22 );
const MODERATOR_COLOUR: Colour   = RGB(0,   173, 3  );
const VIP_COLOUR: Colour         = RGB(244, 5,   185);  
const FOUNDER_COLOUR: Colour     = RGB(170, 64,  213);  
const SUBSCRIBER_COLOUR: Colour  = RGB(130, 5,   180);  
const BITS_COLOUR: Colour        = RGB(193, 178, 17 );
const DEFAULT_COLOUR: Colour     = RGB(255, 255, 255);
const UNKNOWN_COLOUR: Colour     = RGB(200, 200, 200);

use twitch_irc::message::RGBColor;
fn _colour(colour: Option<RGBColor>) -> Colour {
    match colour {
        Some(c) => RGB(c.r, c.g, c.b),
        None => Colour::White,
    }
}

fn _bcolour(badges: &[String]) -> Colour {
    if badges.is_empty() {
        return DEFAULT_COLOUR;
    }

    match badges[0].as_str() {
        "broadcaster" => BROADCASTER_COLOUR,
        "moderator"   => MODERATOR_COLOUR,
        "vip"         => VIP_COLOUR,
        "founder"     => FOUNDER_COLOUR,
        "subscriber"  => SUBSCRIBER_COLOUR,
        "bits"        => BITS_COLOUR,
        &_ => UNKNOWN_COLOUR,
    }
}

fn _badge(badges: &[String]) -> Vec<(String, Colour)>{
    return badges.iter()
        .fold(Vec::new(), |mut acc, b| {
            acc.push(match b.as_str() {
                "broadcaster" => (String::from("brd"), BROADCASTER_COLOUR),
                "moderator"   => (String::from("mod"), MODERATOR_COLOUR),
                "vip"         => (String::from("vip"), VIP_COLOUR),
                "founder"     => (String::from("fnd"), FOUNDER_COLOUR),
                "subscriber"  => (String::from("sub"), SUBSCRIBER_COLOUR),
                "bits"        => (String::from("bit"), BITS_COLOUR),
                &_ => (String::from(&b[0..3]), UNKNOWN_COLOUR),
            }); acc
        });
}

fn _mbadge(badges: &[String]) -> Vec<(String, Colour)>{
    return badges.iter()
        .fold(Vec::new(), |mut acc, b| {
            acc.push(match b.as_str() {
                "broadcaster" => (String::from("B"), BROADCASTER_COLOUR),
                "moderator"   => (String::from("m"), MODERATOR_COLOUR),
                "vip"         => (String::from("v"), VIP_COLOUR),
                "founder"     => (String::from("f"), FOUNDER_COLOUR),
                "subscriber"  => (String::from("s"), SUBSCRIBER_COLOUR),
                "bits"        => (String::from("b"), BITS_COLOUR),
                &_ => (String::from("?"), UNKNOWN_COLOUR),
            }); acc
        });
}

fn _fbadge(badges: &[String]) -> Vec<(String, Colour)>{
    return badges.iter()
        .fold(Vec::new(), |mut acc, b| {
            acc.push((b.to_string(), match b.as_str() {
                "broadcaster" => BROADCASTER_COLOUR,
                "moderator"   => MODERATOR_COLOUR,
                "vip"         => VIP_COLOUR,
                "founder"     => FOUNDER_COLOUR,
                "subscriber"  => SUBSCRIBER_COLOUR,
                "bits"        => BITS_COLOUR,
                &_ => UNKNOWN_COLOUR,
            })); acc
        });
}
