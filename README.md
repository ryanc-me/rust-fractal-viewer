# Rust Fractal Viwer

A small personal project that I worked on while reading [Programming Rust](https://www.oreilly.com/library/view/programming-rust-2nd/9781492052586/), which features an example Mandelbrot set renderer.


## Running

```bash
git clone git@github.com:ryanc-me/rust-fractal-viewer.git
cd rust-fractal-viewer
cargo run
```

 * Click and drag to translate around the scene
 * Scroll in/out to zoom

## WebGPU

This branch uses the [`wgpu`](https://crates.io/crates/wgpu) crate to render fractals on the GPU, and includes a sample shader for the Mandelbrot set. The view transforms (min/max in the complex plane, zoom level, etc) are all calculated on the CPU side and passed in via a uniform. The renderer doesn't implement any kind of caching, so the full set is rendered each frame. The sample shader uses (I think) a pretty naive algorithm, and assigns colors based on the escape time, where the time is converted into degrees (from 0 - 720 deg), taken as the hue component of an HSV value, and converted to RGB.

Unfortunately, the GPU world (at least my built-in graphics) lacks support for `f64`, so the maximum zoom is a bit underwhelming.
