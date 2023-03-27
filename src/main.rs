#![feature(str_split_remainder)]
//https://platform.openai.com/docs/api-reference/completions
//https://platform.openai.com/docs/guides/chat/chat-vs-completions

use clap::Parser;
use openai_rust;
use reedline::{Reedline, Signal, ReedlineEvent, EditCommand, KeyCode, KeyModifiers};
use std::borrow::Cow;
use anyhow::Result;
use openai_rust::futures_util::{Stream, StreamExt};
use std::io::Write;
use std::fs::File;
use colored::Colorize;

#[derive(Parser)]
#[command(author, version, about = "Access OpenAI's models from the command line", long_about = None)]
struct Args {
    /// Your API key
    #[arg(short, long, required = true, env = "OPENAI_API_KEY")]
    api_key: String,
    
    /// Use vim keybinds (instead of emacs)
    #[arg(short, long)]
    vim: bool,
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


    // Here we set up the keybinds
    let edit_mode: Box<dyn reedline::EditMode>;
    if args.vim {
        let mut keybdings_normal = reedline::default_vi_normal_keybindings();
        let keybindings_insert = reedline::default_vi_insert_keybindings();

        keybdings_normal.add_binding(
            KeyModifiers::ALT,
            KeyCode::Enter,
            ReedlineEvent::Edit(vec![EditCommand::InsertNewline])
        );

        edit_mode = Box::new(reedline::Vi::new(keybdings_normal, keybindings_insert));
    } else {
        let mut keybindings = reedline::default_emacs_keybindings();
        
        keybindings.add_binding(
            KeyModifiers::ALT,
            KeyCode::Enter,
            ReedlineEvent::Edit(vec![EditCommand::InsertNewline])
        );

        edit_mode = Box::new(reedline::Emacs::new(keybindings));
    }


    let mut line_editor = Reedline::create().with_edit_mode(edit_mode);    

    loop {
        //println!("{}", &state.prompt);
        let sig = line_editor.read_line(&state);
        match sig {
            Ok(Signal::Success(input)) => {
                if input.starts_with('!') {
                    handle_command(&mut state, &input);
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
fn handle_command(state: &mut State, input: &str) {
    let mut split_input = input.split(' ');
    let cmd = &split_input.next().unwrap()[1..];
    let args = split_input.remainder().unwrap_or_default();
    match cmd {
        "debug" => {
            state.debug = !state.debug;
            println!("Debug mode is {}", if state.debug {"on"} else {"off"});
        },
        "model" => {
            if args.is_empty() {
                println!("You need to specify the model you want");
            } else {
                state.model = args.to_owned();
            }
        },
        "system" => {
            state.history.push(openai_rust::chat::Message {
                role: "system".to_owned(),
                content: args.to_owned(),
            });
        },
        "save" => {
            let Ok(json) = serde_json::to_string(&state.history) else {
                println!("Failed to serialize history");
                return;
            };
            let name = if !args.is_empty() {
                args
            } else {
                if state.name_of_prompt.is_some() {
                    state.name_of_prompt.as_ref().unwrap()
                } else {
                    println!("I need a name to save this conversation as");
                    return;
                }
            };
            match dirs::data_dir() {
                Some(mut path) => {
                    path.push("openai-cli");
                    if let Err(err) = std::fs::create_dir_all(&path) {
                        println!("Failed to create data directory {:?}, {}", path, err);
                        return;
                    }
                    path.push(format!("{}.json", name));
                    match File::create(&path) {
                        Ok(mut file) => {
                            if let Err(err) = file.write(json.as_bytes()) {
                                println!("Failed to write to file {:?}, {}", path, err);
                                return;
                            }
                            state.name_of_prompt = Some(name.to_owned());
                            println!("Saved");
                        },
                        Err(err) => println!("Failed to open file {:?}, {}", path, err),
                    }
                },
                None => {
                    println!("I am not sure where to save this data");
                }
            }
        },
        "load" => {
            // get filename from args
            if args.is_empty() {
                println!("I need the name of the conversation you wish to load");
                return;
            }
            let name = args;
            match dirs::data_dir() {
                Some(mut path) => {
                    path.push("openai-cli");
                    path.push(format!("{}.json", name));
                    match std::fs::read(&path) {
                        Ok(data) => {
                            if let Ok(history) = serde_json::from_slice(&data) {
                                state.history = history;
                                state.name_of_prompt = Some(name.to_owned());
                            } else {
                                println!("Failed to parse JSON");
                            }
                        },
                        Err(err) => println!("Failed to open file {:?}, {}", path, err),
                    }
                },
                None => {
                    println!("Not sure what data directory to read form")
                }
            }
        },
        "history" => {
            for msg in &state.history {
                let role = match msg.role.as_str() {
                    "user" => "User".green().bold().underline(),
                    "assistant" => "Assistant".yellow().bold().underline(),
                    "system" => "System".red().bold().underline(),
                    _ => msg.role.bold().underline()
                };
                println!("{}\n{}\n", role, msg.content);
            }
        },
        "clear" => {
            state.history.clear();
            println!("History cleared");
        },
        _ => {
            println!("Unknown command");
        }
    }
}
