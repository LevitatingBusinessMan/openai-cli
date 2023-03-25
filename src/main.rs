#![feature(str_split_remainder)]
//https://platform.openai.com/docs/api-reference/completions
//https://platform.openai.com/docs/guides/chat/chat-vs-completions

use clap::Parser;
use openai_rust;
use reedline::{Reedline, Signal};
use std::borrow::Cow;
use anyhow::Result;
use openai_rust::futures_util::{Stream, StreamExt};
use std::io::Write;
use std::fs::File;

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

                    let res = send_chat_streaming(&client, &mut state).await;
                    match res {
                        Ok(mut stream) => {
                            let mut response = String::new();
                            while let Some(events) = stream.next().await {
                                for event in events.unwrap() {
                                    let delta = event.choices[0].delta.content.as_ref().unwrap_or(&"".to_owned()).to_owned();
                                    response += &delta;
                                    print!("{}", delta);
                                    std::io::stdout().flush().unwrap();
                                }
                            }
                            state.history.push(openai_rust::chat::Message {
                                role: "assistant".to_owned(),
                                content: response
                            });
                            if state.debug {
                                eprintln!("\n{:?}", state.history);
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

async fn _send_chat(client: &openai_rust::Client, state: &mut State) -> Result<String> {
    let args = openai_rust::chat::ChatArguments::new(&state.model, state.history.clone());
    let res = client.create_chat(args).await?;
    let msg = &res.choices[0].message;
    state.history.push(msg.clone());
    return Ok(msg.content.clone());
}

async fn send_chat_streaming(client: &openai_rust::Client, state: &mut State) -> Result<impl Stream<Item = Result<Vec<openai_rust::chat::stream::ChatResponseEvent>, anyhow::Error>>> {
    let args = openai_rust::chat::ChatArguments::new(&state.model, state.history.clone());
    let res = client.create_chat_stream(args).await?;
    return Ok(res);
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
        "save" => {
            let Ok(json) = serde_json::to_string(&state.history) else {
                return Some("Failed to serialize history".to_owned());
            };
            let name = if !args.is_empty() {
                args
            } else {
                if state.name_of_prompt.is_some() {
                    state.name_of_prompt.as_ref().unwrap()
                } else {
                    return Some("I need a name to save this conversation as".to_owned())
                }
            };
            match dirs::data_dir() {
                Some(mut path) => {
                    path.push("openai-cli");
                    if let Err(err) = std::fs::create_dir_all(&path) {
                        return Some(format!("Failed to create data directory {:?}, {}", path, err)).to_owned();
                    }
                    path.set_file_name(format!("{}.json", name));
                    match File::create(&path) {
                        Ok(mut file) => {
                            if let Err(err) = file.write(json.as_bytes()) {
                                return Some(format!("Failed to write to file {:?}, {}", path, err)).to_owned();
                            }
                            state.name_of_prompt = Some(name.to_owned());
                            Some("Saved".to_owned())
                        },
                        Err(err) => Some(format!("Failed to open file {:?}, {}", path, err)).to_owned()
                    }
                },
                None => {
                    Some("I am not sure where to save this data".to_owned())
                }
            }
        },
        "load" => {
            // get filename from args
            if args.is_empty() {
                return Some("I need the name of the conversation you wish to load".to_owned());
            }
            let name = args;
            match dirs::data_dir() {
                Some(mut path) => {
                    path.push("openai-cli");
                    path.set_file_name(format!("{}.json", name));
                    match std::fs::read(&path) {
                        Ok(data) => {
                            if let Ok(history) = serde_json::from_slice(&data) {
                                state.history = history;
                                state.name_of_prompt = Some(name.to_owned());
                                None
                            } else {
                                Some("Failed to parse JSON".to_owned())
                            }
                        },
                        Err(err) => Some(format!("Failed to open file {:?}, {}", path, err))
                    }
                },
                None => {
                    Some("Not sure what data directory to read form".to_owned())
                }
            }
        },
        _ => {
            return Some("Unknown command".to_owned());
        }
    }
}
