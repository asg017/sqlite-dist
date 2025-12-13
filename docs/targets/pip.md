# Package SQLite Extensions for Python with `sqlite-dist`


The `pip` target in `sqlite-dist` will generate Python wheels for loadable SQLite extensions.
These wheels can then be uploaded to [PyPi](https://pypi.org/), allowing Python developers to 
easily `pip install` your SQLite extension. 

As an example, 


```toml
[package]
name = "sqlite-sample"
license = "MIT"
homepage = "https://…"
repo = "https://…"
description = "…"
authors = ["…"]

[targets]
pip = {}
```
