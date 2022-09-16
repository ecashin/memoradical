# Memoradical

## Trying

You can try out Memoradical at the URL below.
You can make and upload your own JSON data based on the example cards.

https://ecashin.github.io/memoradical

Please see the online help
before clicking the "Memoradical" button
when visiting the app.

## Building

Building the web app requires a Rust installation
with cargo, the wasm target, and trunk.
For details, please check out the [yew](https://yew.rs) documentation
on getting started.

To build and serve, use ...

    RUSTFLAGS=--cfg=web_sys_unstable_apis trunk serve

Or you can `RUSTFLAGS=--cfg=web_sys_unstable_apis trunk build`
and copy the generated files to your web server.
Unless it's at the root of the server,
you will need to edit the links inside `dist/index.html`,
so that `/` becomes `./` in the links.
There is a convenience script you can use for that,
`relativize-resources.sh`.

The extra flag is needed to enable the unstable parts
of `web-sys` that provide access to the browser clipboard.

## Single Tab

Please avoid using memoradical in multiple tabs in the same browser
using the same URL.

There is only one local storage area for memoradical on a browser
visiting a URL where memoradical is served.
The app keeps you from accidentally losing data by stopping
with a fatal error if it detects that another tab has modified the cards
after they were loaded.

In that case, the only functionality available is the copy button,
which you can use to copy the data to your clipboard for offline
backup and examination.

## VsCode

You can set `rust.rustflags` in your `settings.json`
to help VsCode to use experimental features.
