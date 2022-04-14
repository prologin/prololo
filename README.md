# prololo

Matrix bot relaying information about our GitHub repositories, our website...

## License

This software is licensed under the [MIT
License](https://spdx.org/licenses/MIT.html).

## Requirements

On a standard Linux distribution, the current external dependencies seem to be:

- CMake
- pkg-config
- OpenSSL

(all required because `prololo` depends on OpenSSL for TLS communication with a
Matrix server).

## Install

Simply run

```sh
cargo install --path .
```

in the root of the project.

Make sure all the [dependencies](#requirements) have been previously installed.

## Configuration

The bot needs a YAML configuration file to start correctly:

```sh
prololo --config config.yaml
```

The contents of the configuration file are described [here](./src/config.rs),
and example configuration can be found [here](./examples/config.yaml).

### Logging

Rocket uses [log](https://github.com/rust-lang/log) and `prololo` + `matrix_sdk`
both use [tracing](https://github.com/tokio-rs/tracing). Both of these can be
configured by setting the `RUST_LOG` variable to either `error`, `warn`, `info`,
`debug`, or `trace`.

Rocket [also has its own
variable](https://rocket.rs/v0.5-rc/guide/configuration/#overview), that should
be set in addition to `RUST_LOG`.

For example:

```sh
RUST_LOG=info cargo run -- --config config.yaml
```

## Development

The simplest way to get a development environment up and running is to use [the
Nix package manager](https://nixos.org) and run `nix develop`.

If you're not using nix, you can manually installed the
[dependencies](#requirements).

### Easy getting started

The bot won't start if it is not connected to a Matrix server. Therefore, follow [this guide](https://matrix-org.github.io/synapse/latest/setup/installation.html) to install the Synapse Matrix server.

Once you're done with it, edit the config file to add the correct homeserver, password, etc.

Then, simply run the bot:

```sh
RUST_LOG=debug cargo run -- --config config.yaml
```

Finally, you can "simulate" GitHub webhooks by using the script in `utils/generic_payload_sender.py` with examples webhooks in `examples/webhooks/github/`.

### Real testing

To fully start testing prololo, you will need:
- a Matrix Homeserver (see [Synapse installation guide](https://matrix-org.github.io/synapse/latest/setup/installation.html))
- a GitHub organization (a simple repo is OK but you won't be able to test **all** webhooks) with webhooks configured (see [this guide](https://docs.github.com/en/developers/webhooks-and-events/webhooks))
- a web server that is publicly accessible (or your own computer with something like [ngrok](https://ngrok.com/) that will redirect the requests)
- a running instance of the Prologin [website](https://github.com/prologin/site) to test website webhooks
