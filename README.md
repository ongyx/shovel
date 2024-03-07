# Shovel

A package manager for Windows, based off of [Scoop].

## Why?

Development and maintenence on Scoop has slowed down significantly, in part due to feature stability.
However, several feature requests are still open and there has not been a new version since 2022.

Therefore, I created Shovel to determine if:
* A Rust implementation would be more efficient than the current PowerShell implementation.
* Said Rust implementation can achieve feature parity.
* The backload of issues/pull requests/feature requests can be feasibly incorporated.

Shovel does *not* intend to completely replace Scoop, but rather to be an experiemental test bed.

## Development

The MSRV (Minimum Supported Rust Version) for Shovel is the newest stable release available, which is `1.76.0` as of 6/3/2024.

The following toolchains are supported:
* x86-64: `x86_64-pc-windows-msvc`
* x86: `i686-pc-windows-msvc`
* ARM64: `aarch64-pc-windows-msvc`

Your mileage may vary with the GNU toolchains.

## License

[MIT](./LICENSE).

[Scoop]: https://github.com/ScoopInstaller/Scoop
