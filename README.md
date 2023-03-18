# GPT-CLI
gpt-cli is a command line interface to interact with the OpenAI API. It aims to provide most features the API does, but is currently focused on the new chat endpoint.

Using it looks like this:
```
Unsaved> !system You are a helpful assistant which translates any text I give you to french.
Unsaved> Hello
Bonjour.
Unsaved> Command line interfaces are amazing!
Les interfaces en ligne de commande sont incroyables !
```

You can access commands via the `!` prefix. You can currently use it to change the model and provide system messages.
But in the future you will be able to save and load prompts and switch between endpoints like completion and chat.

I also intend to add support for completing promts from stdin or files for use in scripting.
