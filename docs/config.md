# `sqlite-dist.toml` Reference

The `sqlite-dist.toml` file is used to configure the behavior of `sqlite-dist` when building and distributing SQLite extensions. Below is a reference guide to the various configuration options available.

## `[package]`

The `[package]` section contains metadata about the package being distributed.

```toml
# sqlite-dist.toml

[package]
name = "sqlite-sample"
license = "MIT OR Apache"
homepage = "https://alexgarcia.xyz/sqlite-sample"
repo = "https://github.com/asg017/sqlite-sample"
description = "A sample SQLite extension."
authors = ["Alex Garcia"]
# Optional. Path (relative to this file) to a markdown README used as the long
# description on registries like PyPI and npm. Defaults to "README.md".
readme = "README.md"
```

The `description`, `homepage`, `authors`, `license`, and `repo` fields, along with
the `readme` long description, are surfaced as package metadata on PyPI and npm.


## `[targets]`


```toml
# sqlite-dist.toml

[package]
# ... see above 

[targets]
pip = {}
```