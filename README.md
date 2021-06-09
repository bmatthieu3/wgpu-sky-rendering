# wgpu-sky-rendering

A tiny example to show how to render allsky projections using wgpu-rs.
Allsky projections are used in many ways to represented the earth/sky.
This method does the projection per-pixel and runs in realtime.

![Aitoff projection rendering of the sky](./screenshot.png)

Supported projections are:
* Aitoff
* Mercator
* Mollweide
* Azimuthal
* Gnomonic

See this [link](https://en.wikipedia.org/wiki/List_of_map_projections) for information about map projections.

## Algorithm

3d world positions for each pixels are precomputed and stored in RGBA float textures.
In the fragment shader:

* We retrieve the 3d world position of the fragment by looking the precomputed texture.
* A rotation matrix is then applied to the 3d world position.
* After that one can retrieve the HEALPix cell in which the position is located. This gives three things: the index of the HEALPix cell, the dx and dy offset within that cell
* Retrieve the good tile texture from the HEALPix cell index.
* Get the good pixel color using the dx, dy offset within that tile texture.

If you want more precision about the algorithm used. See my talk at ADASS 2020 at this [link](https://www.youtube.com/watch?v=TILtJOiiRoc).

## Usage:

You can use the left and right keyboard touches to change the projections.

### From Source

```sh
# Clone the repo
$ git clone https://github.com/bmatthieu3/wgpu-sky-rendering.git
$ cd wgpu-sky-rendering

# Compile
$ cargo build --release

# Run
$ cargo run --release
```
