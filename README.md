# prololo

Matrix bot relaying information about our GitHub repositories, our website...

## License

This software is licensed under the [MIT
License](https://spdx.org/licenses/MIT.html).

## Development

The simplest way to get a development environment up and running is to use [the
Nix package manager](https://nixos.org) and run `nix develop`.

### Requirements

On a standard Linux distribution, the current external dependencies seem to be:

- CMake
- pkg-config
- OpenSSL

(all required because `prololo` depends on OpenSSL for TLS communication with a
Matrix server).

### Configuration

The bot needs a YAML configuration file to start correctly:

```sh
prololo --config config.yaml
```

The contents of the configuration file are described [here](./src/config.rs).

### Logging

Rocket uses [log](https://github.com/rust-lang/log) and `prololo` + `matrix_sdk`
both use [tracing](https://github.com/tokio-rs/tracing). Both of these can be
configured by setting the `RUST_LOG` variable to either `error`, `warn`, `info`,
`debug`, or `trace`.

Rocket [also has its own
variable](https://rocket.rs/v0.5-rc/guide/configuration/#overview), that should
be set in addition to `RUST_LOG`.
