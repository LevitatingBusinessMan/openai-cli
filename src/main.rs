use clap::Parser;
use openai_gpt_rs::client::Client;
use reedline::{DefaultPrompt, Reedline, Signal};

#[derive(Parser)]
#[command(author, version, about = "Access OpenAI's models from the command line", long_about = None)]
struct Args {
    /// Your API key
    #[arg(short, long, required = true)]
    api_key: String,
}

fn main() {
    let args = Args::parse();
    let client = Client::new(&args.api_key);

    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::default();
    

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                println!("We processed: {}", buffer);
            }
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("\nAborted!");
                break;
            }
            x => {
                println!("Event: {:?}", x);
            }
        }
    }
}
