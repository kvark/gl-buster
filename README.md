## GL Buster

This is a simple program that tests against a few known GL driver issues for [WebRender](https://github.com/servo/webrender) needs. It uses [glow](https://github.com/grovesNL/glow) GL bindings on [surfman](https://github.com/servo/surfman) window-less context.

Example output:
```
Init with renderer: AMD Radeon Pro 460 OpenGL Engine
Test: swizzle
	Relevant extensions: GL_ARB_texture_swizzle
	textureSize: PASS
Test: PBO uploads
	sanity copy at the origin: PASS
	copy at (128, 0, 0) by offset 16384 with stride 4: FAIL [0, 0, 0, 0]
Done
```
## Swizzling

Swizzling can break the texture unit meta-data with Intel 4000 GPUs on Mac:
- Intel HD Graphics 4000 OpenGL Engine

## PBO uploads

On macOS, uploading PBO data with some offsets and origins doesn't work on AMD cards:
- AMD Radeon Pro 460 OpenGL Engine

