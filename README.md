# Bevy Terrain

![GitHub](https://img.shields.io/github/license/Ku95/bevy_terrain)
![Crates.io](https://img.shields.io/crates/v/bevy_terrain)
![docs.rs](https://img.shields.io/docsrs/bevy_terrain)
![Discord](https://img.shields.io/discord/999221999517843456?label=discord)

A terrain rendering plugin for the bevy game engine.

![Screenshot 2022-06-06 at 12 22 40](https://user-images.githubusercontent.com/51823519/172163568-828cce24-c6d8-42ad-91d1-d4f4ce34eebf.png)


This plugin is still in early development.

Join the Bevy Terrain [Discord server](https://discord.gg/7mtZWEpA82) for help or feedback.

## Examples
Currently there are two examples. 

The basic one showcases the different debug views of the terrain. See controls down below.

The advanced one showcases how to use the Bevy material system for texturing, 
as well as how to add additional terrain attachments.
Use the `A` Key to toggle between the custom material and the albedo attachment.

Before running the examples you have to preprocess the terrain data this may take a while.
Once the data is preprocessed you can disable it by commenting out the preprocess line.

## Debug Controls

- `W` - toggle wireframe
- `M` - toggle mesh morph
- `A` - toggle alpha
- `N` - toggle full nodes (or circular lod)
- `S` - toggle lighting
- `D` - toggle density based mesh
- `P` - show patches
- `L` - show LOD
- `U` - show UVs
- `X` - decrease patch scale
- `Q` - increase patch scale
- `I` - decrease view distance
- `O` - increase view distance

<!---
## Supported Bevy Versions

| `bevy_terrain` | `bevy` |
|----------------|--------|
| 0.1.0          | 0.8    |
--->
 

## License
Bevy Terrain is dual-licensed under either

* MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT)
* Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

at your option.