# cargo-vendor-one
Create vendored copies of specific packages and update Cargo.toml to use the copy.

## Installation
```
cargo install --git https://github.com/Seraphin-/cargo-vendor-one
```

## Usage
```
Usage: cargo vendor-one package1[@version1] [package2[@version2] ...]
```

Packages will be copied to `vendor` and the `patch` section of `Cargo.toml` will be added or updating.

## Credits
Forked off code from `cargo-patch`.