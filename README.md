# Shovel 🦀

A package manager for Windows, based off of [Scoop].

## Why?

While the current Powershell implmentation of Scoop works just fine, its speed leaves a lot to be desired.
Therefore, I started this project with two goals in mind:

* Improve the performance of Scoop operations, with some QoL on the side.
* Maintain compatibility with existing Scoop installs.

Shovel does *not* intend to completely replace Scoop, but aims to provide an alternate implementation.
Users should be able to use Scoop and Shovel interchangably.

## Contribution

There are quite a few features to cover, so I'd appreciate PRs for those that are still unimplemented!
Please refer to the [TODO](./TODO.md) list for details.

## Development

The MSRV (Minimum Supported Rust Version) for Shovel is the newest stable release available, which is `1.76.0` as of 6/3/2024.

The following toolchains are supported:
* x86-64: `x86_64-pc-windows-msvc`
* x86: `i686-pc-windows-msvc`
* ARM64: `aarch64-pc-windows-msvc`

GNU toolchains should work, although I haven't tested them yet.

## License

Unless attributed below, all other files are licensed under [MIT](./LICENSE).

### Scoop

These files are dual-licensed under [the Unlicense and MIT].
* [buckets.json](https://github.com/ScoopInstaller/Scoop/blob/master/buckets.json)
* [schema.json](https://github.com/ScoopInstaller/Scoop/blob/master/schema.json)

[Scoop]: https://github.com/ScoopInstaller/Scoop
[the Unlicense and MIT]: https://github.com/ScoopInstaller/Scoop/blob/master/LICENSE