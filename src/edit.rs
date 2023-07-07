use std::io::{Read, Write};
use std::fs::File;
use clap::Parser;

#[derive(Parser)]
pub struct EditArgs {
    #[arg(value_hint=clap::ValueHint::FilePath)]
    pub file: String,
    #[arg(num_args=1..)]
    pub instruction: Vec<String>,

    // this should be based on presence
    #[arg(short, long, help="Create a new file")]
    pub new: Option<bool>,
}

pub async fn edit_mode(file: &str, instruction: String, client: openai_rust::Client) {
    let mut file = File::options().create(true).read(true).write(true).append(false).open(file).expect("Failed to open file");
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    let args = openai_rust::edits::EditArguments::new("text-davinci-edit-001",  buf, instruction.to_owned());
    let response = client.create_edit(args).await.expect("Failed to retrieve response from OpenAI");
    println!("{response}");
    file.write_all(response.to_string().as_bytes()).expect("Failed to write to file");
}
