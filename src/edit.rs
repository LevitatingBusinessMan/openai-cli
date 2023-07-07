use std::io::{Read, Write, Seek};
use std::fs::File;
use clap::Parser;
use std::process::Stdio;
use inquire::Confirm;

#[derive(Parser)]
pub struct EditArgs {
    #[arg(value_hint=clap::ValueHint::FilePath, help = "The file to create or edit")]
    pub file: String,

    #[arg(num_args=1.., trailing_var_arg=true, help = "The edit instructions for gpt")]
    pub instruction: Vec<String>,

    #[arg(short, long, help="Create a new file, do not read the original", action = clap::ArgAction::SetTrue)]
    pub new: Option<bool>,

    #[arg(value_hint=clap::ValueHint::ExecutablePath, env = "EXTERNAL_DIFF", long, default_value = "diff", help = "Diff tool to use")]
    pub diff: String,
}

pub async fn edit_mode(args: &EditArgs, client: openai_rust::Client) {
    println!("{:?}", args.instruction);
    let mut file = File::options().create(true).read(true).write(true).append(false).open(&args.file).expect("Failed to open file");
    let mut original = String::new();
    file.read_to_string(&mut original).unwrap();

    let edit_args = openai_rust::edits::EditArguments {
        model: "text-davinci-edit-001".to_owned(),
        input: if !args.new.unwrap_or(false) { Some(original) } else { None },
        instruction: args.instruction.join(" "),
        n: None,
        temperature: None,
        top_p: None,
    };

    let response = client.create_edit(edit_args).await.expect("Failed to retrieve response from OpenAI");
    println!("{}", response);

    let diff_path = std::env::var("EXTERNAL_DIFF").unwrap_or("diff".to_owned());
    let mut diff_proc = std::process::Command::new(diff_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .arg(&args.file)
        .arg("-")
        .arg("--color=auto")
        .spawn()
        .expect("Failed to spawn diff process");

    diff_proc.stdin.take().expect("Could not get stdin of diff process")
        .write(response.to_string().as_bytes())
        .expect("Failed to write to stdin of diff process");

    diff_proc.wait().expect("Failed to wait for diff process");

    let ans = Confirm::new("Do you want to apply these changes?")
    .with_default(false)
    .prompt();

    match ans {
        Ok(true) => {
            // I could also repoen the file but whatever
            file.rewind().expect("Failed to rewind file");
            file.set_len(0).expect("Failed to truncate file");
            file.write(response.to_string().as_bytes()).expect("Failed to write to file");
            println!("File written");
        },
        Err(_) | Ok(false) => {},
    }

}
