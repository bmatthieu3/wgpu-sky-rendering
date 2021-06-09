// shader.frag
#version 440

layout(location=0) in vec2 pos_cs;
layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_world_pos;
layout(set = 0, binding = 1) uniform sampler s_world_pos;
layout(set = 0, binding = 2) uniform texture2D t_map;
layout(set = 0, binding = 3) uniform sampler s_map;
layout(set = 0, binding = 4)
uniform RotationMatrix {
    mat4 rot;
};
const float TWICE_PI = 6.28318530718f;
const float PI = 3.141592653589793f;
const float FOUR_OVER_PI = 1.27323954474f;
const float TRANSITION_Z = 0.66666666666f;
const float TRANSITION_Z_INV = 1.5f;

int quarter(vec2 p) {
    int x_neg = int(p.x < 0.0f);
    int y_neg = int(p.y < 0.0f);
    int q = (x_neg + y_neg) | (y_neg << 1);
    return q;
}

float xpm1(vec2 p) {
    bool x_neg = (p.x < 0.0f);
    //debug_assert!(x_neg <= 1);
    bool y_neg = (p.y < 0.0f);
    //debug_assert!(y_neg <= 1);
    // The purpose it to have the same numerical precision for each base cell
    // by avoiding subtraction by 1 or 3 or 5 or 7
    float lon = atan(abs(p.y), abs(p.x));
    //debug_assert!(0.0 <= lon && lon <= PI / 2.0);
    float x02 = lon * FOUR_OVER_PI;
    //debug_assert!(0.0 <= x02 && x02 <= 2.0);
    if (x_neg != y_neg) { // Could be replaced by a sign copy from (x_neg ^ y_neg) << 32
        return 1.0f - x02;
    } else {
        return x02 - 1.0f;
    }
}

float one_minus_z_pos(vec3 p) {
    //debug_assert!(z > 0.0);
    float d2 = dot(p.xy, p.xy); // z = sqrt(1 - d2) AND sqrt(1 - x) = 1 - x / 2 - x^2 / 8 - x^3 / 16 - 5 x^4/128 - 7 * x^5/256

    if (d2 < 1e-1f) { // <=> dec > 84.27 deg
        return d2 * (0.5f + d2 * (0.125f + d2 * (0.0625f + d2 * (0.0390625f + d2 * 0.02734375f))));
    }
    return 1.0f - p.z;
}

float one_minus_z_neg(vec3 p) {
    //debug_assert!(z < 0.0);
    float d2 = dot(p.xy, p.xy); // z = sqrt(1 - d2) AND sqrt(1 - x) = 1 - x / 2 - x^2 / 8 - x^3 / 16 - 5 x^4/128 - 7 * x^5/256
    if (d2 < 1e-1f) { // <=> dec < -84.27 deg
        // 0.5 * d2 + 0.125 * d2 * d2
        return d2 * (0.5f + d2 * (0.125f + d2 * (0.0625f + d2 * (0.0390625f + d2 * 0.02734375f))));
    }
    return p.z + 1.0f;
}

// Z-Order curve projection.
int ij2z(int i, int j) {
    int i1 = i | (j << 16);

    int j1 = (i1 ^ (i1 >> 8)) & 0x0000FF00;
    int i2 = i1 ^ j1 ^ (j1 << 8);

    int j2 = (i2 ^ (i2 >> 4)) & 0x00F000F0;
    int i3 = i2 ^ j2 ^ (j2 << 4);

    int j3 = (i3 ^ (i3 >> 2)) & 0x0C0C0C0C;
    int i4 = i3 ^ j3 ^ (j3 << 2);

    int j4 = (i4 ^ (i4 >> 1)) & 0x22222222;
    int i5 = i4 ^ j4 ^ (j4 << 1);

    return i5;
}

struct HashDxDy {
    int idx;
    float dx;
    float dy;
};

// Returns the cell number (hash value) associated with the given position on the unit sphere, 
// together with the offset `(dx, dy)` on the Euclidean plane of the projected position with
// respect to the origin of the cell (South vertex).
// # Inputs:
// - `depth` in `[0, 14]` (so that and HEALPix cell number can be stored on an unsigned integer)
// - `x`: in `[-1.0, 1.0]`
// - `y`: in `[-1.0, 1.0]`
// - `z`: in `[-1.0, 1.0]`
// # Output
// - the cell number (hash value) associated with the given position on the unit sphere,
//   in `[0, 12*nside^2[`
// - `dx`: the positional offset $\in [0, 1[$ along the south-to-east axis
// - `dy`: the positional offset $\in [0, 1[$ along the south-to-west axis
// # WARNING
// - The function assumes, without checking, that the input vector is a unit vector 
//   (hence `x^2 + y^2 + z^2 = 1`) !!
// - Operations being made on simple precision float, the precision is lower than `~0.2 arcsec` only!!
// - At depth 13, the precision on `(dx, dy)` is better than `(1/512, 1/512)`, i.e. 2e-3.
HashDxDy hash_with_dxdy(int depth, vec3 p) {
    //assert!(depth <= 14);
    //assert!(-1.0 <= x && x <= 1.0);
    //assert!(-1.0 <= y && y <= 1.0);
    //assert!(-1.0 <= z && z <= 1.0);
    //debug_assert!(1.0 - (x * x + y * y + z * z) < 1e-5);
    // A f32 mantissa contains 23 bits.
    // - it basically means that when storing (x, y) coordinates,
    //   we can go as deep as depth 24 (or maybe 25)
    
    int nside = 1 << depth;
    float half_nside = float(nside) * 0.5f;

    float x_pm1 = xpm1(p.xy);
    int q = quarter(p.xy);

    int d0h = 0;
    vec2 p_proj = vec2(0.f);
    if (p.z > TRANSITION_Z) {
        // North polar cap, Collignon projection.
        // - set the origin to (PI/4, 0)
        float sqrt_3_one_min_z = sqrt(3.0f * one_minus_z_pos(p));
        p_proj = vec2(x_pm1 * sqrt_3_one_min_z, 2.0f - sqrt_3_one_min_z);
        d0h = q;
    } else if (p.z < -TRANSITION_Z) {
        // South polar cap, Collignon projection
        // - set the origin to (PI/4, -PI/2)
        float sqrt_3_one_min_z = sqrt(3.0f * one_minus_z_neg(p));
        p_proj = vec2(x_pm1 * sqrt_3_one_min_z, sqrt_3_one_min_z);
        d0h = q + 8;
    } else {
        // Equatorial region, Cylindrical equal area projection
        // - set the origin to (PI/4, 0)               if q = 2
        // - set the origin to (PI/4, -PI/2)           if q = 0
        // - set the origin to (0, -TRANSITION_LAT)    if q = 3
        // - set the origin to (PI/2, -TRANSITION_LAT) if q = 1
        // let zero_or_one = (x_cea as u8) & 1;
        float y_pm1 = p.z * TRANSITION_Z_INV;
        // |\2/|
        // .3X1.
        // |/0\|
        int q01 = int(x_pm1 > y_pm1);  // 0/1
        //debug_assert!(q01 == 0 || q01 == 1);
        int q12 = int(x_pm1 >= -y_pm1); // 0\1
        //debug_assert!(q12 == 0 || q12 == 1);
        int q03 = 1 - q12; // 1\0
        //let q13 = q01 ^ q12; debug_assert!(q13 == 0 || q13 == 1);
        int q1 = q01 & q12; // = 1 if q1, 0 else
        //debug_assert!( q1 == 0 ||  q1 == 1);
        // x: xcea - 0 if q3 | xcea - 2 if q1 | xcea - 1 if q0 or q2
        //let x_proj = x_pm1 - ((q01 + q12) as i8 - 1) as f32;
        // y: y - 0 if q2 | y - 1 if q1 or q3 | y - 2 if q0 
        //let y_proj = y_pm1 + (q01 + q03) as f32;
        p_proj = vec2(
            x_pm1 - float(q01 + q12 - 1),
            y_pm1 + float(q01 + q03)
        );
        // d0h: +8 if q0 | +4 if q3 | +5 if q1
        d0h = ((q01 + q03) << 2) + ((q + q1) & 3);
    }

    // Coords inside the base cell
    float x = (half_nside * (p_proj.x + p_proj.y));
    float y = (half_nside * (p_proj.y - p_proj.x));
    int i = int(x);
    int j = int(y);

    return HashDxDy(
        (d0h << (depth << 1)) | ij2z(i, j),
        x - float(i),
        y - float(j)
    );
}

vec4 get_color(vec3 pos) {
    HashDxDy result = hash_with_dxdy(0, pos.zxy);
    int idx = result.idx;
    vec2 uv = vec2(result.dx, result.dy);

    vec2 tq = vec2((float(idx) + uv.y)/12.0, uv.x);

    return texture(sampler2D(t_map, s_map), tq);
    //return vec4(1.0, uv.x, uv.y, 1.0);
}

void main() {
    // Retrieve the position from the texture
    vec3 pos_ws = texture(sampler2D(t_world_pos, s_world_pos), pos_cs).rgb;
    // Rotate it
    vec3 rotated_p = vec3(rot * vec4(pos_ws, 1.0));

    f_color = get_color(rotated_p);
}
 