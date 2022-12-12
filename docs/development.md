# Development Status Bevy Terrain

This document assesses the current status of the `bevy_terrain` plugin.
I built this plugin as part of my bachelor thesis, which focused on rendering large-scale terrains.
The thesis and its project can be found [here](https://github.com/kurtkuehnert/terrain_renderer).

For that, I set out to solve two key problems of terrain rendering. 
For one, I developed the Uniform Distance-Dependent Level of Detail (UDLOD) algorithm to represent the terrain geometry, and for another, 
I came up with the Chunked Clipmap data structure used to represent the terrain data. 
Both are implemented as part of bevy terrain and work quite well for rendering large-scale terrains.

Now that I have finished my thesis I would like to continue working on this project and extend its capabilities. 
The topic of terrain rendering is vast, and thus I can not work on all the stuff at once.
In the following, I will list a couple of features that I would like to integrate into this crate in the future. 
I will probably not have the time to implement all of them by myself, so if you are interested please get in touch, and let us work on them together. 
Additionally, there are still plenty of improvements, bug fixes, and optimizations to be completed on the already existing implementation.

## Features

- Procedural Texturing
- Shadow Rendering
- Real-Time Editing
- Collision
- Path-Finding
- Spherical Terrain

### Procedural Texturing

Probably the biggest missing puzzle piece of this plugin is support for procedural texturing using splat maps or something similar. 
Currently, texturing has to be implemented manually in the terrain shader (see the advanced example for reference). 
I would like to support this use case in a more integrated manner in the future. Unfortunately, 
I am not familiar with the terrain texturing systems of other engines (e.g. Unity, Unreal, Godot) 
or have any experience texturing and building my own terrains. 
I would greatly appreciate it if anyone can share some requirements for this area of terrain rendering. 
Also, a prototype of a custom texturing system would be a great resource to develop further ideas.

### Shadow Rendering

Another important capability that is currently missing is the support for large-scale shadow rendering. 
This would be probably implemented using cascading shadow maps or a similar method.
Currently, Bevy itself does not implement a system we could use for this yet. 
Regardless, I think reusing Bevy’s implementation would be the best choice in the future.

### Real-Time Editing

One of the most interesting problems that need to be solved before `bevy_terrain` can be used for any serious project is the editing of the terrain data in real time. 
This is not only important for sculpting the terrain of your game, but also for texturing, vegetation placement, etc.
This is going to be my next focus area and I would like to discuss designs and additional requirements with anyone interested.

### Collision

Same as for shadow rendering, Bevy does not have a built-in physics engine yet. For now, the de-facto standard is the rapier physics engine. 
Integrating the collision of the terrain with rapier would enable many types of games and is a commonly requested feature.

### Path-Finding

Similar to collision, path-finding is essential for most games. I have not investigated this field at all yet, but I am always interested in your ideas.

### Spherical Terrain

I think that with a little design work the current two-dimensional terrain rendering method could be extended to the spherical terrain.
However, I am unsure how much of the existing code could be extended and reused. Maybe planet rendering would require its entirely separate crate.
