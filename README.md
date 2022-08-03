# Bevy Terrain
A terrain rendering plugin for the bevy game engine.

![Screenshot 2022-06-06 at 12 22 40](https://user-images.githubusercontent.com/51823519/172163568-828cce24-c6d8-42ad-91d1-d4f4ce34eebf.png)

This plugin is still in early development.
For a simple example see the examples folder.

Documentation and usage instructions coming soon.

## Examples
Currently there are two examples. 

The basic one showcases the different debug views of the terrain. See controls down below.

The advanced one showcases how to use the Bevy material system for texturing.

Before running the examples you have to preprocess the terrain data first, by running the prepocess example.

`cargo run --example preprocess`

## License
Bevy Terrain is dual-licensed under either

* MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT)
* Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Controls

- W - toggle wireframe
- M - toggle mesh morph (circular transitions)
- A - toggle alpha (if loaded)
- N - toggle full nodes (or circular lod)
- S - toggle light
- P - show patches
- L - show LOD
- U - show UVs
- X - decrease patch scale
- Q - increase patch scale
- I - decrease view distance
- O - increase view distance
