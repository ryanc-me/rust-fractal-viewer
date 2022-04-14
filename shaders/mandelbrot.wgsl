
struct VertexInput {
    [[location(0), interpolate(flat)]] position: vec3<f32>;
    [[location(1)]] color: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
};

struct Complex {
    re: f32;
    im: f32;
};
struct ComplexPair {
    a: Complex;
    b: Complex;
};
struct CameraState {
    width: f32;
    height: f32;
    origin: Complex;
    scale: f32;
    zoom: f32;
    min: Complex;
    max: Complex;
};

[[group(0), binding(0)]]
var<uniform> camera: CameraState;


[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.position = vec4<f32>(model.position, 1.0);
    return out;
}


fn cmul(a: Complex, b: Complex) -> Complex {
    var res: Complex;
    res.re = (a.re * b.re) - (a.im * b.im);
    res.im = (a.re * b.im) + (a.im * b.re);
    return res;    
}
fn cadd(a: Complex, b: Complex) -> Complex {
    var res: Complex;
    res.re = a.re + b.re;
    res.im = a.im + b.im;
    return res;    
}

fn complex_norm_sqr(c: Complex) -> f32 {
    var res: f32;
    res = (c.re * c.re) + (c.im * c.im);
    return res;
}

fn hsv_to_rgb(hue: f32, sat: f32, val: f32) -> vec4<f32> {
    var C = (val / 100.0) * (sat / 100.0);
    var X = C * (1.0 - abs((hue / 60.0) % 2.0 - 1.0));
    var m = (val / 100.0) - C;

    var r: f32;
    var g: f32;
    var b: f32;
    if (0.0 <= hue && hue < 60.0) {
        r = C;
        g = X;
        b = 0.0;
    }
    else if (60.0 <= hue && hue < 120.0) {
        r = X;
        g = C;
        b = 0.0;
    }
    else if (120.0 <= hue && hue < 180.0) {
        r = 0.0;
        g = C;
        b = X;
    }
    else if (180.0 <= hue && hue < 240.0) {
        r = 0.0;
        g = X;
        b = C;
    }
    else if (240.0 <= hue && hue < 300.0) {
        r = X;
        g = 0.0;
        b = C;
    }
    else if (300.0 <= hue && hue < 360.0) {
        r = C;
        g = 0.0;
        b = X;
    }

    return vec4<f32>(r + m, g + m, b + m, 1.0);
}

fn calculate_limits(width: f32, height: f32, scale: f32, origin: Complex, zoom: f32) -> ComplexPair {
    var ratio_x: f32;
    var ratio_y: f32;
    if (width > height) {
        ratio_x = 1.0;
        ratio_y = width / height;
    }
    else {
        ratio_x = height / width;
        ratio_y = 1.0;
    }

    let min_re = -scale / 2.0 / ratio_x;
    let max_re =  scale / 2.0 / ratio_x;
    let min_im = -scale / 2.0 / ratio_y;
    let max_im =  scale / 2.0 / ratio_y;

    var min: Complex;
    var max: Complex;
    min.re = (min_re / zoom) + origin.re;
    min.im = (min_im / zoom) + origin.im;
    max.re = (max_re / zoom) + origin.re;
    max.im = (max_im / zoom) + origin.im;

    var res: ComplexPair;
    res.a = min;
    res.b = max;
    return res;
}

fn pixel_to_point(x: f32, y: f32, width: f32, height: f32, min: Complex, max: Complex) -> Complex {
    var w = max.re - min.re;
    var h = min.im - max.im;
    var res: Complex;
    res.re = min.re + x * w / width;
    res.im = min.im - y * h / height; 
    return res;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var s = pixel_to_point(in.position.x, in.position.y, camera.width, camera.height, camera.min, camera.max);
    var z: Complex;
    z.re = 0.0;
    z.im = 0.0;
    var max: f32 = 255.0;
    for (var i: f32 = max; i >= 0.0; i = i - 1.0) {
        if (complex_norm_sqr(z) > 4.0) {
            break;
        }
        z = cadd(cmul(z, z), s);
    }
    var n = (i / max);
    return hsv_to_rgb(n * 720.0 % 360.0, 100.0, n * 100.0);
}
