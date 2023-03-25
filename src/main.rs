#![feature(str_split_remainder)]
//https://platform.openai.com/docs/api-reference/completions
//https://platform.openai.com/docs/guides/chat/chat-vs-completions

use clap::Parser;
use openai_rust;
use reedline::{Reedline, Signal};
use std::borrow::Cow;
use anyhow::Result;

#[derive(Parser)]
#[command(author, version, about = "Access OpenAI's models from the command line", long_about = None)]
struct Args {
    /// Your API key
    #[arg(short, long, required = true, env = "OPENAI_API_KEY")]
    api_key: String,
}

struct State {
    /// Name of the saved or built-in prompt
    name_of_prompt: Option<String>,

    /// Complete prompt to send to GPT
    //prompt: String,

    /// History of the messages
    history: Vec<openai_rust::chat::Message>,

    /// The model used
    model: String,

    debug: bool,
}

impl reedline::Prompt for State {
    fn render_prompt_left(&self) -> Cow<str> {
        if let Some(promptname) = &self.name_of_prompt {
            return Cow::Borrowed(promptname);
        } else {
            return Cow::Borrowed("Unsaved");
        }
    }

    fn render_prompt_right(&self) -> Cow<str> {
        Cow::Owned(format!("({})", &self.model).to_owned())
    }

    fn render_prompt_indicator(&self, _prompt_mode: reedline::PromptEditMode) -> Cow<str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("-")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: reedline::PromptHistorySearch) -> Cow<str> {
        Cow::Borrowed("search: ")
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = openai_rust::Client::new(&args.api_key);

    let mut state = State {
        name_of_prompt: None,
        //prompt: String::new(),
        history: vec![],
        model: "gpt-3.5-turbo".to_owned(),
        debug: false,
    };

    let mut line_editor = Reedline::create();    

    loop {
        //println!("{}", &state.prompt);
        let sig = line_editor.read_line(&state);
        match sig {
            Ok(Signal::Success(input)) => {
                if input.starts_with('!') {
                    let res = handle_command(&mut state, &input);
                    if let Some(res) = res {
                        println!("{res}");
                    }
                } else {

                    state.history.push(openai_rust::chat::Message {
                        role: "user".to_owned(),
                        content: input
                    });

                    let res = send_chat(&client, &mut state).await;
                    match res {
                        Ok(msg) => {
                            println!("{}", msg.trim());
                            if state.debug {
                                eprintln!("{:?}", state.history);
                            }
                        },
                        Err(e) => {
                            println!("{e}");
                        }
                    }
                }
            }
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("Quitting");
                break;
            }
            x => {
                println!("Event: {:?}", x);
            }
        }
    }
}

async fn send_chat(client: &openai_rust::Client, state: &mut State) -> Result<String> {
    let args = openai_rust::chat::ChatArguments::new(&state.model, state.history.clone());
    let res = client.create_chat(args).await?;
    let msg = &res.choices[0].message;
    state.history.push(msg.clone());
    return Ok(msg.content.clone());
}

/// Handle a command and return the response
fn handle_command(state: &mut State, input: &str) -> Option<String> {
    let mut split_input = input.split(' ');
    let cmd = &split_input.next().unwrap()[1..];
    let args = split_input.remainder().unwrap_or_default();
    match cmd {
        "debug" => {
            state.debug = !state.debug;
            return Some(format!("Debug mode is {}", if state.debug {"on"} else {"off"}));
        },
        "model" => {
            state.model = args.to_owned();
            return None;
        },
        "system" => {
            state.history.push(openai_rust::chat::Message {
                role: "system".to_owned(),
                content: args.to_owned(),
            });
            return None;
        },
        _ => {
            return Some("Unknown command".to_owned());
        }
    }
}
