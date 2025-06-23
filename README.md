<div align="center">
  <h1>Ollama applet for COSMIC Desktop</h1>
  <br>
  <div>
    <img alt="chat" src="https://github.com/cosmic-utils/cosmic-ext-applet-ollama/blob/main/screenshots/chat.png" width="280">
    <img alt="settings" src="https://github.com/cosmic-utils/cosmic-ext-applet-ollama/blob/main/screenshots/settings.png" width="280">
  </div>
</div>

Before using this applet, you must have Ollama installed on your system. To do this, run this in your terminal:

```sh
curl -fsSL https://ollama.com/install.sh | sh
```

Source: [Ollama Github](https://github.com/ollama/ollama?tab=readme-ov-file#linux)

After installing Ollama. Pull some models, you would like to use with chat, for example

```sh
ollama pull mistral
```

More models you can find in library: [Ollama/Library](https://ollama.com/library)

# Installing this applet

Clone the repository, and use [just](https://github.com/casey/just)

If you don't have `just` installed, it is available in PopOS repository,
so you can install it with `apt`

```sh
sudo apt install just
```

or for Fedora

```sh
sudo dnf install just
```

Now you can clone repo and install applet.

```sh
git clone https://github.com/cosmic-utils/cosmic-ext-applet-ollama.git
cd cosmic-ext-applet-ollama
```

## Building

Run just:

```sh
just build-release
```

### Installing

```sh
sudo just install
```

Done

From now, you will be able to add applet to your desktop panel/dock
and chat with different models in real time :)

Cheers!  

## Known wgpu issue

There are currently some rendering issues with the `wgpu` libcosmic features
in some (older?) gpus. This doesn't affect Ollama, only the applet.
If you are affected by this, you can build and install it with this feature disabled:

```sh
just build-no-wgpu
sudo just install
```
