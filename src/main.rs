// https://github.com/rust-lang/rust/issues/77998
//#![feature(str_split_remainder)]

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
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::{SyntaxSet, SyntaxReference};
use syntect::util::as_24_bit_terminal_escaped;
use lazy_static::lazy_static;

#[derive(Parser)]
#[command(author, version, about = "Access OpenAI's models from the command line", long_about = None)]
struct Args {
    /// Your API key
    #[arg(short, long, required=true, env="OPENAI_API_KEY", hide_env_values=true)]
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

lazy_static! {
    static ref SS: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref TS: ThemeSet = ThemeSet::load_defaults();
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
                    handle_command(&client, &mut state, &input).await;
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
                                    let delta = event.to_string();
                                    response += &delta;
                                    if state.debug {
                                        println!("{:?} - {:?}", delta, get_mode(&response));
                                    } else {
                                        print!("{}", beautify_response(&response , delta));
                                    }
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

#[derive(Debug)]
enum PrintMode {
    Normal,
    CodeBlock(SyntaxReference),
    Bold,
}

fn get_mode(response: &str) -> PrintMode {
    let mut count = 0;

    let mut last_triple = None;

    // this loops counts the triple backticks
    for i in 2..response.len() {
        if &response[i-2..i+1] == "```" {
            count += 1;
            last_triple = Some(i);
        }
    }

    if count % 2 == 1 {
        let code = &response[last_triple.unwrap()..];
        let first_line = code.split('\n').next().unwrap();
        let syntax = SS.find_syntax_by_first_line(first_line);
        return PrintMode::CodeBlock(syntax.unwrap_or_else(|| SS.find_syntax_plain_text()).to_owned());
    }

    let mut count = 0;
    
    // this loops counts the single backticks
    for i in 0..response.len() {

        if response.chars().nth(i).unwrap() != '`' {
            continue;
        }

        if i > 0 {
            if response.chars().nth(i-1).unwrap_or('a') == '`' {
                continue;
            }
        }
        
        if response.chars().nth(i+1).unwrap_or('a') == '`' {
            continue;
        }

        count += 1;
    }

    if count % 2 == 1 {
        return PrintMode::Bold;
    }
    
    return PrintMode::Normal;
}

/// This functions handles any formatting that can be done on the output.
/// It's a bit limited because the responses are streamed.
fn beautify_response(response: &str, delta: String) -> String {
    return match get_mode(response) {
        PrintMode::Bold => delta.bold().to_string(),
        PrintMode::CodeBlock(syntax) => {
            let last_line = response.split('\n').last().unwrap_or("");
            let mut h = HighlightLines::new(&syntax, &TS.themes["base16-ocean.dark"]);
            let ranges = h.highlight_line(last_line, &SS).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
            return "\r".to_owned() + &escaped;
        },
        PrintMode::Normal => delta
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
/// Async so it can do API calls
async fn handle_command(client: &openai_rust::Client, state: &mut State, input: &str) {
    let mut split_input = input.split(' ');
    let cmd = &split_input.next().unwrap()[1..];
    
    // https://github.com/rust-lang/rust/issues/77998
    // let args = split_input.remainder().unwrap_or_default();
    
    let args = split_input.collect::<Vec<&str>>().join(" ");

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
                &args
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
            state.name_of_prompt = None;
            println!("History cleared");
        },
        "models" => {
            match client.list_models().await {
                Ok(mut models) => {
                    models.sort_by(|a, b| a.id.cmp(&b.id));
                    for model in models { println!("{}", model.id) };
                }
                Err(err) => println!("{err}"),
            }
        },
        "undo" => {
            match state.history.pop() {
                Some(msg) => {
                    match msg.role.as_str() {
                        // Popping user would currently not be possible.
                        "assistant" => {
                            match  state.history.pop() {
                                Some(msg2) => println!("Undid {} and {} message", msg2.role, msg.role),
                                None => println!("Undid {} message", msg.role),
                            }
                            
                        },
                        _ =>  println!("Undid {} message", msg.role),
                    }
                },
                None => println!("No messages to undo"),
            }
        },
        _ => {
            println!("Unknown command");
        }
    }
}
