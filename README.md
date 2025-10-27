# audio-chat-rs


A real-time websocket based audio-to-audio chat application built in Rust.
It streams audio from a web client to the OpenAI Realtime API and streams the audio response back.
This repository is a rust rewrite of the original [openai-audio-ws](https://github.com/ShawonAshraf/openai-audio-ws).

## dev
### env setup
Create a .env file in the root of the project with your OpenAI API key:

```
OPENAI_API_KEY=sk-your-openai-api-key-goes-here
```

### server

```bash
cargo run
```

This will start the server will start on 0.0.0.0:8000. 

### browser

Navigate to http://localhost:8000 in your browser. 
Click "Connect", then "Start Recording" to begin.
