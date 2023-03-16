//https://platform.openai.com/docs/api-reference/completions
//https://platform.openai.com/docs/guides/chat/chat-vs-completions

use clap::Parser;
use reedline::{Reedline, Signal};
use std::borrow::Cow;

#[derive(Parser)]
#[command(author, version, about = "Access OpenAI's models from the command line", long_about = None)]
struct Args {
    /// Your API key
    #[arg(short, long, required = true)]
    api_key: String,
}

struct State {
    /// Name of the saved or built-in prompt
    name_of_prompt: Option<String>,

    /// Complete prompt to send to GPT
    prompt: String,

    /// The model used
    model: String
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

    fn render_prompt_indicator(&self, prompt_mode: reedline::PromptEditMode) -> Cow<str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("-")
    }

    fn render_prompt_history_search_indicator(&self, history_search: reedline::PromptHistorySearch) -> Cow<str> {
        Cow::Borrowed("search: ")
    }
}

fn main() {
    let args = Args::parse();
    let client = openai_gpt_rs::client::Client::new(&args.api_key);

    let mut state = State {
        name_of_prompt: None,
        prompt: String::new(),
        model: "text-davinci-003".to_owned(),
    };

    let mut line_editor = Reedline::create();    

    loop {
        let sig = line_editor.read_line(&state);
        match sig {
            Ok(Signal::Success(buffer)) => {
                state.prompt += &buffer;
                let res = perform_completion(&client, &state);
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

async fn perform_completion(client: &openai_gpt_rs::client::Client, state: &State) -> Result<String, String>  {
    let completion_args = openai_gpt_rs::args::CompletionArgs::new(
        &state.prompt,
        None,
        None,
        None,
        None
    );

    let result = client.create_completion(&completion_args).await;
    match result {
        Ok(res) => {
            println!("{:?}",res.resp);
            return Ok("this is a stupid library".to_owned());
        }
        Err(err) => {
            return Err("Eh you got error".to_owned());
        }
    }
}
