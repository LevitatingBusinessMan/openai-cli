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
    pub new: bool,

    #[arg(value_hint=clap::ValueHint::ExecutablePath, env = "EXTERNAL_DIFF", long, default_value = "diff", help = "Diff tool to use")]
    pub diff: String,

    // TODO: autocomplete
    #[arg(short, long, help = "Model to use", default_value = "gpt-3.5-turbo-16k")]
    pub model: String,
}

pub async fn edit_mode(args: &EditArgs, client: openai_rust::Client) {
    let instruction = args.instruction.join(" ");
    let mut file = File::options().create(true).read(true).write(true).append(false).open(&args.file).expect("Failed to open file");
    let mut original = String::new();
    file.read_to_string(&mut original).unwrap();

    // let edit_args = openai_rust::edits::EditArguments {
    //     model: "text-davinci-edit-001".to_owned(),
    //     input: if !args.new.unwrap_or(false) { Some(original) } else { None },
    //     instruction: args.instruction.join(" "),
    //     n: None,
    //     temperature: None,
    //     top_p: None,
    // };

    // let response = client.create_edit(edit_args).await.expect("Failed to retrieve response from OpenAI");

    

    let messages = if original.is_empty() || args.new {
        vec![
            openai_rust::chat::Message {
                role: "system".to_owned(),
                content: "The user will give you instructions for a program. You shall reply only with the content of that program without further instructions. Do not use codeblocks.".to_owned(),
            },
            openai_rust::chat::Message {
                role: "user".to_owned(),
                content: instruction,
            },
        ]
    } else {
        vec![
            openai_rust::chat::Message {
                role: "system".to_owned(),
                content: "Apply changes to the text or code supplied by the user.".to_owned(),
            },
            openai_rust::chat::Message {
                role: "user".to_owned(),
                content: original,
            },
            openai_rust::chat::Message {
                role: "user".to_owned(),
                content: instruction,
            },
        ]
    };

    println!("{:?}", messages);

    let chat_args = openai_rust::chat::ChatArguments::new(args.model.clone(), messages);
    let response = client.create_chat(chat_args).await.expect("Failed to get a response from openai");

    let mut diff_proc = std::process::Command::new(&args.diff)
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
