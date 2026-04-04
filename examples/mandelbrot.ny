// Mandelbrot Set — ASCII Art Generator
// Demonstrates: nested loops, floating-point math, extern FFI, casting
//
// Outputs a 78x36 ASCII mandelbrot set to stdout.
// Characters represent iteration depth: " .:-=+*#%@" (space = in set)
extern {
    fn putchar(c: i32) -> i32;
}

fn mandelbrot(cr: f64, ci: f64, max_iter: i32) -> i32 {
    zr :~ f64 = 0.0;
    zi :~ f64 = 0.0;
    i :~ i32 = 0;
    while i < max_iter {
        zr2 := zr * zr;
        zi2 := zi * zi;
        if zr2 + zi2 > 4.0 {
            return i;
        }
        zi = 2.0 * zr * zi + ci;
        zr = zr2 - zi2 + cr;
        i += 1;
    }
    return max_iter;
}

fn main() -> i32 {
    width := 78;
    height := 36;
    max_iter := 80;
    // Palette: 10 characters from sparse to dense
    // Space = in the set (max iterations reached)
    // Index:  0    1    2    3    4    5    6    7    8    9
    // Char:  ' '  '.'  ':'  '-'  '='  '+'  '*'  '#'  '%'  '@'
    p0 := 32; // ' '
    p1 := 46; // '.'
    p2 := 58; // ':'
    p3 := 45; // '-'
    p4 := 61; // '='
    p5 := 43; // '+'
    p6 := 42; // '*'
    p7 := 35; // '#'
    p8 := 37; // '%'
    p9 := 64; // '@'
    y :~ i32 = 0;
    while y < height {
        x :~ i32 = 0;
        while x < width {
            // Map pixel to complex plane: real [-2.5, 1.0], imag [-1.2, 1.2]
            cr := -2.5 + x as f64 * 3.5 / width as f64;
            ci := -1.2 + y as f64 * 2.4 / height as f64;
            iter := mandelbrot(cr, ci, max_iter);
            // Map iteration count to palette character
            // Points IN the set (max_iter) get '@', escaped points get lighter chars
            ch :~ i32 = p0;
            if iter >= max_iter {
                ch = p0;
            } else {
                idx := iter * 10 / max_iter;
                if idx == 0 {
                    ch = p1;
                } else if idx == 1 {
                    ch = p2;
                } else if idx == 2 {
                    ch = p3;
                } else if idx == 3 {
                    ch = p4;
                } else if idx == 4 {
                    ch = p5;
                } else if idx == 5 {
                    ch = p6;
                } else if idx == 6 {
                    ch = p7;
                } else if idx == 7 {
                    ch = p8;
                } else if idx == 8 {
                    ch = p9;
                } else {
                    ch = p9;
                }
            }
            putchar(ch);
            x += 1;
        }
        putchar(10); // newline
        y += 1;
    }
    return 0;
}
