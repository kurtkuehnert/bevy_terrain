# Bevy Terrain

![GitHub](https://img.shields.io/github/license/Ku95/bevy_terrain)
![Crates.io](https://img.shields.io/crates/v/bevy_terrain)
![docs.rs](https://img.shields.io/docsrs/bevy_terrain)
![Discord](https://img.shields.io/discord/999221999517843456?label=discord)

Bevy Terrain is a plugin for rendering terrains with the Bevy game engine.

![](https://user-images.githubusercontent.com/51823519/202845032-0537e929-b13c-410b-8072-4c5b5df9830d.png)
(Data Source: Federal Office of Topography, [©swisstopo](https://www.swisstopo.admin.ch/en/home.html))

**Warning:** This plugin is still in early development, so expect the API to change and possibly break you existing code.

Bevy terrain was developed as part of my [bachelor thesis](https://github.com/kurtkuehnert/terrain_renderer) on the topic of large-scale terrain rendering.
Now that this project is finished I am planning on adding more features related to game development and rendering virtual worlds.
If you would like to help me build an extensive open-source terrain rendering library for the Bevy game engine, feel free to contribute to the project.
Also, join the Bevy Terrain [Discord server](https://discord.gg/7mtZWEpA82) for help, feedback, or to discuss feature ideas.

## Examples
Currently, there are two examples. 

The basic one showcases the different debug views of the terrain. See controls down below.

The advanced one showcases how to use the Bevy material system for texturing, 
as well as how to add additional terrain attachments.
Use the `A` Key to toggle between the custom material and the albedo attachment.

Before running the examples you have to preprocess the terrain data this may take a while.
Once the data is preprocessed you can disable it by commenting out the preprocess line.

## Documentation
The `docs` folder contains a high-level [implementation overview](https://github.com/kurtkuehnert/bevy_terrain/blob/main/docs/implementation.md),
as well as, the [development status](https://github.com/kurtkuehnert/bevy_terrain/blob/main/docs/development.md), enumerating the features that I am planning on implementing next, of the project.
If you would like to contribute to the project this is a good place to start. Simply pick an issue/feature and discuss the details with me on Discord or GitHub.
I would also recommend you to take a look at my [thesis](https://github.com/kurtkuehnert/terrain_renderer/blob/main/Thesis.pdf).
There I present the basics of terrain rendering (chapter 2), common approaches (chapter 3) and a detailed explanation of method used by `bevy_terrain` (chapter 4).

## Debug Controls
These are the debug controls of the plugin.
Use them to fly over the terrain, experiment with the quality settings and enter the different debug views.

- `T` - toggle camera movement
- move the mouse to look around
- press the arrow keys to move the camera horizontally
- use `PageUp` and `PageDown` to move the camera vertically
- use `Home` and `End` to increase/decrease the camera's movement speed

- `W` - toggle wireframe view
- `P` - toggle tile view
- `L` - toggle lod view
- `U` - toggle uv view
- `C` - toggle node view
- `D` - toggle mesh morph
- `A` - toggle albedo
- `B` - toggle base color black / white
- `S` - toggle lighting
- `G` - toggle filtering bilinear / trilinear + anisotropic
- `F` - freeze frustum culling
- `H` - decrease tile scale
- `J` - increase tile scale
- `N` - decrease grid size
- `E` - increase grid size
- `I` - decrease view distance
- `O` - increase view distance

<!---
## Supported Bevy Versions

| `bevy_terrain` | `bevy` |
|----------------|--------|
| 0.1.0          | 0.9    |
--->
 

## License
Bevy Terrain is dual-licensed under either

* MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT)
* Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
