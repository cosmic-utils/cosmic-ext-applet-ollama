<div align="center">
  <h1>COSMIC Applet for Ollama chat</h1>
  <br>
  <div>
    <img alt="chat" src="https://github.com/elevenhsoft/cosmic-ext-applet-ollama/blob/main/screenshots/chat.png" width="280">
    <img alt="settings" src="https://github.com/elevenhsoft/cosmic-ext-applet-ollama/blob/main/screenshots/settings.png" width="280">
  </div>
</div>

Before using this applet, you must have Ollama installed on your system. To do this, run this in your terminal:

`curl -fsSL https://ollama.com/install.sh | sh`

Source: [Ollama Github](https://github.com/ollama/ollama?tab=readme-ov-file#linux)

After installing Ollama. Pull some models, you would like to use with chat, for example

`ollama pull llama3`

More models you can find in library: https://ollama.com/library

# Installing this applet

Clone the repository, and use [just](https://github.com/casey/just)

If you don't have `just` installed, it is available in PopOS repository, so you can install it with `apt`

`sudo apt install just`

Now you can clone repo and install applet.

`git clone https://github.com/elevenhsoft/cosmic-ext-applet-ollama.git`

`cd cosmic-ext-applet-ollama`

### Building

Run just:

`just`

### Installing

`sudo just install`

Done

From now, you will be able to add applet to your desktop panel/dock and chat with different models in real time :)

Cheers!
