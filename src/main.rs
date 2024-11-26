use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;
use twitch_irc::message::ServerMessage;

use ansi_term::Colour::{RGB, self};

use std::sync::Arc;
use std::process::exit;

#[derive(Default, Debug)]
struct Args {
	flags:   u16,
	file:    Option<&'static str>,
	names:   Vec<&'static str>,
	ignore:  Vec<&'static str>,
}

const COLOUR:  u16 = 1 << 0;
const BCOLOUR: u16 = 1 << 1;
const BADGE:   u16 = 1 << 2;
const MBADGE:  u16 = 1 << 3;
const FBADGE:  u16 = 1 << 4;
const BITS:    u16 = 1 << 5;
const ORIGIN:  u16 = 1 << 6;
const TIME:    u16 = 1 << 7;
const QUIET:   u16 = 1 << 8;

const USAGE: &str = "Usage: tchat [-bBcCdFhoqtTV] [-f file] [-i username] [username ...]";
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
    This option may be repeated to filter multiple users.\n";

fn parse_args<I: std::iter::Iterator<Item = String>>(mut args: I) -> Args {
	let mut out = Args::default();

	while let Some(arg) = args.next() {
		arg.strip_prefix('-').map(|c| c.chars().for_each(|c| match c {
			'h' => {
				println!("{}\n\n{}", USAGE, HELP_MESSAGE);
				exit(0);
			},

			'V' => {
				println!("{}", env!("CARGO_PKG_VERSION"));
				exit(0);
			},

			'c' => out.flags |= COLOUR,
			'C' => out.flags |= BCOLOUR,
			'b' => out.flags |= BADGE,
			'B' => out.flags |= MBADGE,
			'F' => out.flags |= FBADGE,
			't' => out.flags |= BITS,
			'o' => out.flags |= ORIGIN,
			'T' => out.flags |= TIME,
			'q' => out.flags |= QUIET,

			'f' => out.file = match args.next() {
				Some(a) => Some(Box::leak(a.clone().into_boxed_str())),
				None    => panic!("Missing argument after `-f`\n {}\n{}", USAGE, ERR_MESSAGE),
			},

			'i' => out.ignore.push(match args.next() {
				Some(a) => Box::leak(a.clone().into_boxed_str()),
				None    => panic!("Missing argument after `-i`\n {}\n{}", USAGE, ERR_MESSAGE),
			}),

			_ => panic!("{}\n{}", USAGE, ERR_MESSAGE),
		})).or_else(|| {
			out.names.push(Box::leak(arg.into_boxed_str()));
			Some(())
		});
	} out
}

const BROADCASTER_COLOUR: Colour = RGB(233, 25,  22 );
const MODERATOR_COLOUR: Colour   = RGB(0,   173, 3  );
const VIP_COLOUR: Colour         = RGB(244, 5,   185);  
const FOUNDER_COLOUR: Colour     = RGB(170, 64,  213);  
const SUBSCRIBER_COLOUR: Colour  = RGB(130, 5,   180);  
const BITS_COLOUR: Colour        = RGB(193, 178, 17 );
const DEFAULT_COLOUR: Colour     = RGB(255, 255, 255);
const UNKNOWN_COLOUR: Colour     = RGB(200, 200, 200);

async fn handle_msg(args: &Args, msg: twitch_irc::message::PrivmsgMessage) {
	#[cfg(debug_assertions)]
	dbg!(&msg);

	if args.ignore.contains(&msg.sender.name.as_str()) { return; }

	let badges: Vec<String> = msg.badges.iter()
		.map(|b| b.name.to_string()).collect();

	let badge_map = |b| match b {
		"broadcaster" => BROADCASTER_COLOUR,
		"moderator"   => MODERATOR_COLOUR,
		"vip"         => VIP_COLOUR,
		"founder"     => FOUNDER_COLOUR,
		"subscriber"  => SUBSCRIBER_COLOUR,
		"bits"        => BITS_COLOUR,
		_             => UNKNOWN_COLOUR,
	};

	let name_colour =
		if args.flags & COLOUR != 0 {
			match msg.name_color {
				Some(c) => RGB(c.r, c.g, c.b),
				None    => Colour::White,
			}.into()
		}
		else if args.flags & BCOLOUR != 0 {
			match badges.as_slice() {
				[] => DEFAULT_COLOUR,
				_  => badge_map(&badges[0]),
			}.into()
		}
		else { None };

	let display_badges =
		if args.flags & BADGE != 0 
			{ badges.iter().map(|b| (String::from(&b[0..3]), badge_map(b))).collect() }
		else if args.flags & MBADGE != 0 { 
			badges.iter().map(|b| (
					match b.as_str() {
						"broadcaster" => "B",
						"moderator"   => "m",
						"vip"         => "v",
						"founder"     => "f",
						"subscriber"  => "s",
						"bits"        => "b",
						&_            => "?", }.to_string(),
					badge_map(b)))
				.collect()
		}
		else if args.flags & FBADGE != 0 
			{ badges.iter().map(|b| (b.to_string(), badge_map(b))).collect() }
		else { Vec::new() };


	let mut message = String::new();

	if args.flags & TIME != 0 {
		message.push_str(&chrono::Local::now().format("%H:%M ").to_string());
	}

	if args.flags & ORIGIN != 0 {
		message.push_str(&format!("[{}]: ", msg.channel_login));
	}

	if args.flags & BITS != 0 && msg.bits.is_some() {
		message.push_str(&BITS_COLOUR.bold().paint(format!("!{}!", msg.bits.unwrap())).to_string());
	}

	display_badges.iter().for_each(|(b, c)|
		message.push_str(&c.bold().paint(format!("|{b}|")).to_string()));

	if !message.is_empty() && !message.ends_with(' ') {
		message.push(' '); 
	}

	match name_colour {
		Some(c) => message.push_str(&c.bold().paint(format!("{}: ", msg.sender.name)).to_string()),
		None    => message.push_str(&format!("{}: ", msg.sender.name)),
	}

	message.push_str(&msg.message_text);
	message.push('\n');

	if args.flags & QUIET == 0 { print!("{message}"); }

	if let Some(file) = args.file {
		let mut file = std::fs::OpenOptions::new()
			.append(true).create(true)
			.open(file).unwrap();

		let mut message = message.as_str();
		let mut out = String::new();
		while let Some(i) = message.find('\x1b') {
			out.push_str(&message[..i]);
			message = &message[i..];
			message = &message[message.find('m').unwrap() + 1..];
		}

		use std::io::Write;
		file.write_all(out.as_bytes()).unwrap();
	};
}

#[tokio::main]
async fn main() {
	std::panic::set_hook(Box::new(|info| {
		eprintln!("{info}");
		exit(1);
	}));

	let args = Arc::new(parse_args(std::env::args().skip(1)));

	if args.names.is_empty() 
		{ panic!("{}\n{}", USAGE, ERR_MESSAGE); }

	#[cfg(debug_assertions)]
	dbg!(&args);

	let (mut message, client) = 
		TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(
			twitch_irc::ClientConfig::default());

	let _args = Arc::clone(&args);
	let handle = tokio::spawn(async move {
		while let Some(msg_recv) = message.recv().await {
			if let ServerMessage::Privmsg(msg) = msg_recv 
				{ handle_msg(&_args, msg).await; }
		}
	});

	args.names.iter().for_each(|name|
		client.join(String::from(*name)).expect("Failed to join"));

	handle.await.unwrap()
}
