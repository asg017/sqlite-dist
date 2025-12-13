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
```


## `[targets]`


```toml
# sqlite-dist.toml

[package]
# ... see above 

[targets]
pip = {}
```