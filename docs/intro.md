# Introduction to `sqlite-dist`

[SQLite extensions](https://sqlite.org/loadext.html) are a powerful way to add custom functions, virtual tables, and virtual filesystems to SQLite. 

*Loadable* SQLite extensions are compiled shared object files (`.so` on Linux, `.dylib` on MacOS, `.dll` on Windows) that can be used in many places  — on laptops, servers, phones, watches, embedded devices. Every programming languages has their own SQLite client libraries. So, making sure your SQLite extensions are available to users across all platforms and programming languages is challenging!

That's where `sqlite-dist` comes in. `sqlite-dist` is a tool meant to be ran during your SQLite extension's release process (ie in Github Actions or other CI/CD workflows). It will package up pre-compiled SQLite extensions for various platforms (ie MacOS ARM, Linux x86_64, Linux ARM, Windows x86_64, etc.) and generate packages for various programming languages. 

Here's an example — say you have created a SQLite extension, and have compiled it for various platforms. 

## Buildintg