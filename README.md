# Memoradical

## Trying

You can try out Memoradical at the URL below.
You can make and upload your own JSON data based on the example cards.

https://ecashin.github.io/memoradical

Please see the online help before clicking "Go"
when visiting the app.

## Building

Building the web app requires a Rust installation
with cargo, the wasm target, and trunk.
For details, please check out the [yew](https://yew.rs) documentation
on getting started.

To build and serve, use ...

    trunk serve

Or you can `trunk build` and copy the generated files to your web server.
Unless it's at the root of the server,
you will need to edit the links inside `dist/index.html`,
so that `/` becomes `./` in the links.
There is a convenience script you can use for that,
`relativize-resources.sh`.
