# openai-cli
[![Crates.io](https://img.shields.io/crates/v/openai-cli)](https://crates.io/crates/openai-cli)

openai-cli is a command line interface to interact with the OpenAI API. It aims to provide most features the API does.
## Chat mode
[![demo](https://asciinema.org/a/587475.svg)](https://asciinema.org/a/587475)
### Chat mode commands
* `!system`: Write a system message, e.g., "You will translate text to french".
* `!save`: Save a conversation.
* `!load`: Load a converstation.
* `!history`: Show the converstation history.
* `!model`: Set the model you want to use, e.g., `!model gpt-3.5-turbo`.
* `!clear`: Clear the current conversation history.
* `!undo`: Undo last message.

I also intend to add support for completing promts from stdin or files for use in scripting.

#### Keybinds
By defeault openai-cli uses emacs style keybinds.

However by using the `--vim` flag, you can use vim style keybinds instead.

You can **write newlines** using `ALT+ENTER`.

## Edit Mode
You can edit files using the edit subcommand.
![edit demo](https://i.imgur.com/bDFsOOG.png)

## Installation
Installation can be done via `cargo install openai-cli`.
