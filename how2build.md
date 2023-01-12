To build this project you will need to install nightly [rust](https://www.rust-lang.org/)

Target's linkers are mentioned in [config.toml](/.cargo/config.toml):
* Windows - [x86_64-w64-mingw32-gcc](https://www.mingw-w64.org/downloads/)
* Linux - [lld](https://pkgs.org/download/lld)

To build use:
`cargo build`

To specify target platform:
`cargo build --target x86_64-pc-windows-gnu`

To get optimized release version:
`cargo build --release`