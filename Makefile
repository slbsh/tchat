build:
	cargo build --release
	mv target/release/tchat ./tchat
	rm -rf target/

install:
	cargo build --release
	sudo mv target/release/tchat /usr/bin/tchat
	rm -rf target/
