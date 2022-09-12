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

## TODO

Here are improvements that would help out.

### Multiple Tabs: Avoid Overwriting Newer Data

The local data is different for each server and client pair.
But if a user has multiple tabs of memoradical open
in the same browser,
visiting the same location,
then it is possible to, e.g., add a card "foo" in one tab (state A),
add a second card "bar" in another tab (state B),
and overwrite state A with state B,
so that the newly stored set of cards lacks "foo".

You might wonder, "Why would they do that!?"
On accident, probably.

A checksum could be used to ensure that the current
persistent data is the same as that which was loaded
earlier.
This method still entails a race condition,
but it would suffice for most human-speed concurrency
and would be a relatively easy enhancement while offering
a lot of protection.

## VsCode

You can set `rust.rustflags` in your `settings.json`
to help VsCode to use experimental features.
