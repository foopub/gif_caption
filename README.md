Be funny and caption gifs in your browser. Point of this is to do it all locally without
sending your data to some server. This is my first Rust app so it's probably full of
spaghetti. 

I'm using the nightly toolchian, but it should work with stable I think. Run using:
```
cargo install trunk wasm-bindgen-cli
rustup target add wasm32-unknown-unknown
trunk serve --release
```

You can make a `fonts/` directory, and put this font in, 
https://fonts.google.com/specimen/Fjalla+One
or just edit the path to another. I'll some better font selection method at some point.
