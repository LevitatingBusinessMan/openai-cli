// https://github.com/rust-lang/rust/issues/77998
//#![feature(str_split_remainder)]

//https://platform.openai.com/docs/api-reference/completions
//https://platform.openai.com/docs/guides/chat/chat-vs-completions

mod edit;
mod chat;
mod models;

use chat::ChatArgs;
use clap::Parser;
use openai_rust;

#[derive(Parser)]
#[command(author, version, about = "Access OpenAI's models from the command line", long_about = None)]
pub struct Args {
    /// Your API key
    #[arg(short, long, required=true, env="OPENAI_API_KEY", hide_env_values=true)]
    api_key: String,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    #[command(about="Start a chat session (default)", trailing_var_arg=true)]
    Chat(chat::ChatArgs),
    #[command(about="Edit or create a file")]
    Edit(edit::EditArgs),
    #[command(about="List all available models")]
    Models(models::ModelsArgs),
    //Ask, (Chat but single resonse answer)
    //Complete, (COmplete a prompt)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = openai_rust::Client::new(&args.api_key);

    match &args.command {
        Some(cmd) => match cmd {
            Command::Edit(args) => {edit::edit_mode(args, client).await;},
            Command::Chat(args) => {chat::chat_mode(args, client).await;},
            Command::Models(args) => {models::models_mode(args, client).await;},
        },
        None => chat::chat_mode(&ChatArgs::default(), client).await
    }
}
