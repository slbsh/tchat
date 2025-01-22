```
Usage: tchat [-bBcCdFhoqtTV] [-f file] [-i username] [username ...]
 
DESCRIPTION
    tchat logs a twitch chat into the terminal, allowing for the monitoring during streams.
    Whenever multiple usernames are specified both of the chats will be monitored simultaneously.
 
    All of the colour options work properly only within truecolor terminals.
    
OPTIONS
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
```


# Installation
- latest stable release of the rust toolchain.  
- openssl or libressl  
`cargo install --path .`
