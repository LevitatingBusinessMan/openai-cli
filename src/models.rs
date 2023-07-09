use clap::Parser;

#[derive(Parser)]
pub struct ModelsArgs {

}

pub async fn models_mode(_args: &ModelsArgs, client: openai_rust::Client) {
    let models = client.list_models().await.expect("Failed to retrieve models");
    for model in models {
        println!("{}", model.id);   
    }
}
