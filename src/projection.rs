// Screen space: pixels space between
// * x_px in [0, width-1]
// * y_px in [0, height-1]

// Homogeneous space
// * x_h in [-1, 1]
// * y_h in [-1, 1]

use crate::math::Float;

use cgmath::Vector4;
pub trait Projection<T: Float> {
    /// World to screen space projection
    fn world_to_normalized_device_space(
        pos_world_space: &Vector4<T>,
        width_screen: T,
        height_screen: T,
    ) -> Option<Vector2<T>> {
        if let Some(pos_clip_space) = Self::world_to_clip_space(pos_world_space) {
            let ndc_to_clip = Self::compute_ndc_to_clip_factor(width_screen, height_screen);

            let pos_normalized_device = Vector2::new(
                pos_clip_space.x / (ndc_to_clip.x),
                pos_clip_space.y / (ndc_to_clip.y),
            );
            Some(pos_normalized_device)
        } else {
            None
        }
    }

    /// Perform a clip to the world space deprojection
    ///
    /// # Arguments
    ///
    /// * ``pos_clip_space`` - The position in the clipping space (orthonorlized space)
    fn clip_to_world_space(pos_clip_space: &Vector2<T>) -> Option<Vector4<T>>;
    /// World to the clipping space deprojection
    ///
    /// # Arguments
    ///
    /// * ``pos_world_space`` - The position in the world space
    fn world_to_clip_space(pos_world_space: &Vector4<T>) -> Option<Vector2<T>>;

    fn is_included_inside_projection(pos_clip_space: &Vector2<T>) -> bool;

    fn is_front_of_camera(pos_world_space: &Vector4<T>) -> bool;

    fn compute_ndc_to_clip_factor(width: T, height: T) -> Vector2<T>;

    fn solve_along_abscissa(y: T) -> Option<(T, T)>;
    fn solve_along_ordinate(x: T) -> Option<(T, T)>;
}

pub struct Aitoff;
pub struct Mollweide;
pub struct Ortho;
pub struct AzimuthalEquidistant;
pub struct Gnomonic;
pub struct Mercator;

use crate::math;
use cgmath::Vector2;
use crate::math::{Vec2, Vec4};
impl<T> Projection<T> for Aitoff
where
    T: Float,
{
    fn compute_ndc_to_clip_factor(width: T, height: T) -> Vector2<T> {
        if width > T::from(2.0).unwrap() * height {
            // reduce width
            Vector2::new(
                T::from(2.0).unwrap() * height / width,
                T::from(2.0).unwrap(),
            )
        } else {
            // reduce height
            Vector2::new(T::one(), width / height)
        }
    }

    fn is_included_inside_projection(pos_clip_space: &Vector2<T>) -> bool {
        // Semi-major axis length
        let a = T::one();
        // Semi-minor axis length
        let b = T::one() * T::from(0.5).unwrap();

        let a2 = a * a;
        let b2 = b * b;
        let px2 = pos_clip_space.x * pos_clip_space.x;
        let py2 = pos_clip_space.y * pos_clip_space.y;

        (px2 * b2 + py2 * a2) < a2 * b2
    }

    fn solve_along_abscissa(y: T) -> Option<(T, T)> {
        let t = T::from(1e-3).unwrap();
        if y.abs() > T::from(0.5).unwrap() {
            None
        } else {
            let x = (T::one() - T::from(4.0).unwrap() * y * y).sqrt();
            Some((-x + t, x - t))
        }
    }
    fn solve_along_ordinate(x: T) -> Option<(T, T)> {
        let t = T::from(1e-3).unwrap();
        if x.abs() > T::one() {
            None
        } else {
            let y = (T::one() - x * x).sqrt() * T::from(0.5).unwrap();
            Some((-y + t, y - t))
        }
    }

    /// View to world space transformation
    ///
    /// This returns a normalized vector along its first 3 dimensions.
    /// Its fourth component is set to 1.
    ///
    /// The Aitoff projection maps screen coordinates from [-pi; pi] x [-pi/2; pi/2]
    ///
    /// # Arguments
    ///
    /// * `x` - in normalized device coordinates between [-1; 1]
    /// * `y` - in normalized device coordinates between [-1; 1]
    fn clip_to_world_space(pos_clip_space: &Vec2<T>) -> Option<Vec4<T>> {
        if Self::is_included_inside_projection(&pos_clip_space) {
            let u = pos_clip_space.x * T::PI() * T::from(0.5).unwrap();
            let v = pos_clip_space.y * T::PI();
            //da uv a lat/lon
            let c = (v * v + u * u).sqrt();

            let (phi, mut theta) = if c != T::zero() {
                let phi = (v * c.sin() / c).asin();
                let theta = (u * c.sin()).atan2(c * c.cos());
                (phi, theta)
            } else {
                let phi = v.asin();
                let theta = u.atan();
                (phi, theta)
            };
            theta *= T::from(2.0).unwrap();

            let pos_world_space = cgmath::Vector4::new(
                theta.sin() * phi.cos(),
                phi.sin(),
                theta.cos() * phi.cos(),
                T::one(),
            );

            Some(pos_world_space)
        } else {
            None
        }
    }

    /// World to screen space transformation
    /// X is between [-1, 1]
    /// Y is between [-0.5, 0.5]
    ///
    /// # Arguments
    ///
    /// * `pos_world_space` - Position in the world space. Must be a normalized vector
    fn world_to_clip_space(pos_world_space: &Vector4<T>) -> Option<Vector2<T>> {
        // X in [-1, 1]
        // Y in [-1/2; 1/2] and scaled by the screen width/height ratio
        //return vec3(X / PI, aspect * Y / PI, 0.f);

        //let pos_world_space = pos_world_space;

        let xyz = pos_world_space.truncate();
        let (theta, delta) = math::xyz_to_radec(&xyz);

        let theta_by_two = theta / T::from(2.0).unwrap();

        let alpha = (delta.0.cos() * theta_by_two.0.cos()).acos();
        let inv_sinc_alpha = if alpha < T::from(1e-3).unwrap() {
            T::one()
        } else {
            alpha / alpha.sin()
        };

        // The minus is an astronomical convention.
        // longitudes are increasing from right to left
        let x = T::from(2.0).unwrap() * inv_sinc_alpha * delta.0.cos() * theta_by_two.0.sin();
        let y = inv_sinc_alpha * delta.0.sin();

        Some(Vector2::new(x / T::PI(), y / T::PI()))
    }

    fn is_front_of_camera(_pos_world_space: &Vector4<T>) -> bool {
        // 2D projections always faces the camera
        true
    }
}

use crate::angle::Angle;
impl<T> Projection<T> for Ortho
where
    T: Float,
{
    fn compute_ndc_to_clip_factor(width: T, height: T) -> Vector2<T> {
        if width > height {
            // reduce width
            Vector2::new(height / width, T::one())
        } else {
            // reduce height
            Vector2::new(T::one(), width / height)
        }
    }

    fn is_included_inside_projection(pos_clip_space: &Vector2<T>) -> bool {
        let px2 = pos_clip_space.x * pos_clip_space.x;
        let py2 = pos_clip_space.y * pos_clip_space.y;

        (px2 + py2) < T::one()
    }

    fn solve_along_abscissa(y: T) -> Option<(T, T)> {
        if y.abs() > T::one() {
            None
        } else {
            let t = T::from(1e-3).unwrap();
            let x = (T::one() - y * y).sqrt();
            Some((-x + t, x - t))
        }
    }
    fn solve_along_ordinate(x: T) -> Option<(T, T)> {
        if x.abs() > T::one() {
            None
        } else {
            let t = T::from(1e-3).unwrap();

            let y = (T::one() - x * x).sqrt();
            Some((-y + t, y - t))
        }
    }

    /// View to world space transformation
    ///
    /// This returns a normalized vector along its first 3 dimensions.
    /// Its fourth component is set to 1.
    ///
    /// The Aitoff projection maps screen coordinates from [-pi; pi] x [-pi/2; pi/2]
    ///
    /// # Arguments
    ///
    /// * `x` - in normalized device coordinates between [-1; 1]
    /// * `y` - in normalized device coordinates between [-1; 1]
    fn clip_to_world_space(pos_clip_space: &Vector2<T>) -> Option<cgmath::Vector4<T>> {
        let xw_2 =
            T::one() - pos_clip_space.x * pos_clip_space.x - pos_clip_space.y * pos_clip_space.y;
        if xw_2 > T::zero() {
            let pos_world_space =
                cgmath::Vector4::new(pos_clip_space.x, pos_clip_space.y, xw_2.sqrt(), T::one());

            Some(pos_world_space)
        } else {
            // Out of the sphere
            None
        }
    }

    /// World to screen space transformation
    ///
    /// # Arguments
    ///
    /// * `pos_world_space` - Position in the world space. Must be a normalized vector
    fn world_to_clip_space(pos_world_space: &cgmath::Vector4<T>) -> Option<Vector2<T>> {
        if pos_world_space.z < T::zero() {
            None
        } else {
            Some(Vector2::new(pos_world_space.x, pos_world_space.y))
        }
    }

    fn is_front_of_camera(pos_world_space: &Vector4<T>) -> bool {
        pos_world_space.z > T::zero()
    }
}

impl<T> Projection<T> for Mollweide
where
    T: Float,
{
    fn compute_ndc_to_clip_factor(width: T, height: T) -> Vector2<T> {
        if width > T::from(2.0).unwrap() * height {
            // reduce width
            Vector2::new(
                T::from(2.0).unwrap() * height / width,
                T::from(2.0).unwrap(),
            )
        } else {
            // reduce height
            Vector2::new(T::one(), width / height)
        }
    }

    fn is_included_inside_projection(pos_clip_space: &Vector2<T>) -> bool {
        // Semi-major axis length
        let a = T::one();
        // Semi-minor axis length
        let b = T::from(0.5).unwrap();

        let a2 = a * a;
        let b2 = b * b;
        let px2 = pos_clip_space.x * pos_clip_space.x;
        let py2 = pos_clip_space.y * pos_clip_space.y;

        (px2 * b2 + py2 * a2) < a2 * b2
    }

    fn solve_along_abscissa(y: T) -> Option<(T, T)> {
        if y.abs() > T::from(0.5).unwrap() {
            None
        } else {
            let x = (T::one() - T::from(4.0).unwrap() * y * y).sqrt();
            Some((-x + T::from(1e-3).unwrap(), x - T::from(1e-3).unwrap()))
        }
    }
    fn solve_along_ordinate(x: T) -> Option<(T, T)> {
        if x.abs() > T::one() {
            None
        } else {
            let y = (T::one() - x * x).sqrt() * T::from(0.5).unwrap();
            let t = T::from(1e-3).unwrap();

            Some((-y + t, y - t))
        }
    }

    /// View to world space transformation
    ///
    /// This returns a normalized vector along its first 3 dimensions.
    /// Its fourth component is set to 1.
    ///
    /// The Aitoff projection maps screen coordinates from [-pi; pi] x [-pi/2; pi/2]
    ///
    /// # Arguments
    ///
    /// * `x` - in normalized device coordinates between [-1; 1]
    /// * `y` - in normalized device coordinates between [-1; 1]
    fn clip_to_world_space(pos_clip_space: &Vector2<T>) -> Option<cgmath::Vector4<T>> {
        if Self::is_included_inside_projection(&pos_clip_space) {
            let y2 = pos_clip_space.y * pos_clip_space.y;
            let four = T::from(4.0).unwrap();
            let two = T::from(2.0).unwrap();
            let k = (T::one() - four * y2).sqrt();

            let theta = T::PI() * pos_clip_space.x / k;
            let delta = ((two * (two * pos_clip_space.y).asin() + four * pos_clip_space.y * k)
                / T::PI())
            .asin();

            // The minus is an astronomical convention.
            // longitudes are increasing from right to left
            let pos_world_space = cgmath::Vector4::new(
                theta.sin() * delta.cos(),
                delta.sin(),
                theta.cos() * delta.cos(),
                T::one(),
            );

            Some(pos_world_space)
        } else {
            None
        }
    }

    /// World to screen space transformation
    /// X is between [-1, 1]
    /// Y is between [-0.5, 0.5]
    ///
    /// # Arguments
    ///
    /// * `pos_world_space` - Position in the world space. Must be a normalized vector
    fn world_to_clip_space(pos_world_space: &Vector4<T>) -> Option<Vector2<T>> {
        // X in [-1, 1]
        // Y in [-1/2; 1/2] and scaled by the screen width/height ratio
        let epsilon = T::from(1e-12).unwrap();
        let max_iter = 10;

        let xyz = pos_world_space.truncate();
        let (lon, lat) = math::xyz_to_radec(&xyz);
        let cst = T::PI() * lat.sin();

        let mut theta = lat.0;
        let mut f = theta + theta.sin() - cst;

        let mut k = 0;
        while f.abs() > epsilon && k < max_iter {
            theta -= f / (T::one() + theta.cos());
            f = theta + theta.sin() - cst;

            k += 1;
        }

        theta /= T::from(2.0).unwrap();

        // The minus is an astronomical convention.
        // longitudes are increasing from right to left
        let x = (lon.0 / T::PI()) * theta.cos();
        let y = T::from(0.5).unwrap() * theta.sin();

        Some(Vector2::new(x, y))
    }

    fn is_front_of_camera(_pos_world_space: &Vector4<T>) -> bool {
        // 2D projections always faces the camera
        true
    }
}

impl<T> Projection<T> for AzimuthalEquidistant
where
    T: Float,
{
    fn compute_ndc_to_clip_factor(width: T, height: T) -> Vector2<T> {
        if width > height {
            // reduce width
            Vector2::new(height / width, T::one())
        } else {
            // reduce height
            Vector2::new(T::one(), width / height)
        }
    }

    fn is_included_inside_projection(pos_clip_space: &Vector2<T>) -> bool {
        let px2 = pos_clip_space.x * pos_clip_space.x;
        let py2 = pos_clip_space.y * pos_clip_space.y;

        (px2 + py2) < T::one()
    }

    fn solve_along_abscissa(y: T) -> Option<(T, T)> {
        if y.abs() > T::one() {
            None
        } else {
            let x = (T::one() - y * y).sqrt();
            let t = T::from(1e-3).unwrap();

            Some((-x + t, x - t))
        }
    }
    fn solve_along_ordinate(x: T) -> Option<(T, T)> {
        if x.abs() > T::one() {
            None
        } else {
            let y = (T::one() - x * x).sqrt();
            let t = T::from(1e-3).unwrap();

            Some((-y + t, y - t))
        }
    }

    /// View to world space transformation
    ///
    /// This returns a normalized vector along its first 3 dimensions.
    /// Its fourth component is set to 1.
    ///
    /// The Aitoff projection maps screen coordinates from [-pi; pi] x [-pi/2; pi/2]
    ///
    /// # Arguments
    ///
    /// * `x` - in normalized device coordinates between [-1; 1]
    /// * `y` - in normalized device coordinates between [-1; 1]
    fn clip_to_world_space(pos_clip_space: &Vector2<T>) -> Option<cgmath::Vector4<T>> {
        // r <= pi
        let x = pos_clip_space.x * T::PI();
        let y = pos_clip_space.y * T::PI();
        let mut r = (x * x + y * y).sqrt();
        if r > T::PI() {
            None
        } else {
            let z = r.cos();
            r = math::sinc_positive(r);

            let pos_world_space = Vector4::new(-x * r, y * r, z, T::one());

            Some(pos_world_space)
        }
    }

    /// World to screen space transformation
    ///
    /// # Arguments
    ///
    /// * `pos_world_space` - Position in the world space. Must be a normalized vector
    fn world_to_clip_space(pos_world_space: &Vector4<T>) -> Option<Vector2<T>> {
        if pos_world_space.z > -T::one() {
            // Distance in the Euclidean plane (xy)
            // Angular distance is acos(x), but for small separation, asin(r)
            // is more accurate.
            let mut r = (pos_world_space.x * pos_world_space.x
                + pos_world_space.y * pos_world_space.y)
                .sqrt();
            if pos_world_space.z > T::zero() {
                // Angular distance < PI/2, angular distance = asin(r)
                r = math::asinc_positive::<T>(r);
            } else {
                // Angular distance > PI/2, angular distance = acos(x)
                r = pos_world_space.z.acos() / r;
            }
            let x = pos_world_space.x * r;
            let y = pos_world_space.y * r;

            Some(Vector2::new(x / T::PI(), y / T::PI()))
        } else {
            Some(Vector2::new(T::one(), T::zero()))
        }
    }

    fn is_front_of_camera(_pos_world_space: &Vector4<T>) -> bool {
        // 2D projections always faces the camera
        true
    }
}

impl<T> Projection<T> for Gnomonic
where
    T: Float,
{
    fn compute_ndc_to_clip_factor(_width: T, _height: T) -> Vector2<T> {
        Vector2::new(T::one(), T::one())
    }

    fn is_included_inside_projection(pos_clip_space: &Vector2<T>) -> bool {
        let px = pos_clip_space.x;
        let py = pos_clip_space.y;

        px > -T::one() && px < T::one() && py > -T::one() && py < T::one()
    }

    fn solve_along_abscissa(y: T) -> Option<(T, T)> {
        if y.abs() > T::one() {
            None
        } else {
            let t = T::from(1e-3).unwrap();

            Some((-T::one() + t, T::one() - t))
        }
    }
    fn solve_along_ordinate(x: T) -> Option<(T, T)> {
        if x.abs() > T::one() {
            None
        } else {
            let t = T::from(1e-3).unwrap();

            Some((-T::one() + t, T::one() - t))
        }
    }

    /// View to world space transformation
    ///
    /// This returns a normalized vector along its first 3 dimensions.
    /// Its fourth component is set to 1.
    ///
    /// The Aitoff projection maps screen coordinates from [-pi; pi] x [-pi/2; pi/2]
    ///
    /// # Arguments
    ///
    /// * `x` - in normalized device coordinates between [-1; 1]
    /// * `y` - in normalized device coordinates between [-1; 1]
    fn clip_to_world_space(pos_clip_space: &Vector2<T>) -> Option<cgmath::Vector4<T>> {
        let x_2d = pos_clip_space.x * T::PI();
        let y_2d = pos_clip_space.y * T::PI();
        let r = x_2d * x_2d + y_2d * y_2d;

        let z = (T::one() + r).sqrt();
        let pos_world_space = Vector4::new(z * x_2d, z * y_2d, z, T::one());

        Some(pos_world_space)
    }

    /// World to screen space transformation
    ///
    /// # Arguments
    ///
    /// * `pos_world_space` - Position in the world space. Must be a normalized vector
    fn world_to_clip_space(pos_world_space: &Vector4<T>) -> Option<Vector2<T>> {
        if pos_world_space.z <= T::from(1e-2).unwrap() {
            // Back hemisphere (z < 0) + diverges near z=0
            None
        } else {
            let pos_clip_space = Vector2::new(
                (pos_world_space.x / pos_world_space.z) / T::PI(),
                (pos_world_space.y / pos_world_space.z) / T::PI(),
            );
            Some(pos_clip_space)
        }
    }

    fn is_front_of_camera(pos_world_space: &Vector4<T>) -> bool {
        // 2D projections always faces the camera
        pos_world_space.z >= T::from(1e-2).unwrap()
    }
}

impl<T> Projection<T> for Mercator
where
    T: Float,
{
    fn compute_ndc_to_clip_factor(_width: T, _height: T) -> Vector2<T> {
        Vector2::new(T::one(), T::from(2.0).unwrap())
    }

    fn is_included_inside_projection(pos_clip_space: &Vector2<T>) -> bool {
        let px = pos_clip_space.x;
        let py = pos_clip_space.y;

        px > -T::one() && px < T::one() && py > -T::one() && py < T::one()
    }

    fn solve_along_abscissa(y: T) -> Option<(T, T)> {
        if y.abs() > T::one() {
            None
        } else {
            let t = T::from(1e-3).unwrap();

            Some((-T::one() + t, T::one() - t))
        }
    }
    fn solve_along_ordinate(x: T) -> Option<(T, T)> {
        if x.abs() > T::one() {
            None
        } else {
            let t = T::from(1e-3).unwrap();
            Some((-T::one() + t, T::one() - t))
        }
    }

    /// View to world space transformation
    ///
    /// This returns a normalized vector along its first 3 dimensions.
    /// Its fourth component is set to 1.
    ///
    /// The Aitoff projection maps screen coordinates from [-pi; pi] x [-pi/2; pi/2]
    ///
    /// # Arguments
    ///
    /// * `x` - in normalized device coordinates between [-1; 1]
    /// * `y` - in normalized device coordinates between [-1; 1]
    fn clip_to_world_space(pos_clip_space: &Vector2<T>) -> Option<cgmath::Vector4<T>> {
        let theta = pos_clip_space.x * T::PI();
        let delta = (pos_clip_space.y.sinh()).atan() * T::PI();

        let pos_world_space = math::radec_to_xyzw(Angle(theta), Angle(delta));

        Some(pos_world_space)
    }

    /// World to screen space transformation
    ///
    /// # Arguments
    ///
    /// * `pos_world_space` - Position in the world space. Must be a normalized vector
    fn world_to_clip_space(pos_world_space: &Vector4<T>) -> Option<Vector2<T>> {
        let (theta, delta) = math::xyzw_to_radec(&pos_world_space);

        Some(Vector2::new(
            theta.0 / T::PI(),
            ((delta.0 / T::PI()).tan()).asinh() as T,
        ))
    }

    fn is_front_of_camera(_pos_world_space: &Vector4<T>) -> bool {
        // 2D projections always faces the camera
        true
    }
}

mod tests {

    #[test]
    fn generate_maps() {
        use super::*;
        use cgmath::InnerSpace;
        use cgmath::Vector2;
        use image::{Rgba, RgbaImage};
        use super::math::Float;
        fn generate_projection_map<P: Projection<f32>>(filename: &str) {
            let (w, h) = (1024.0, 1024.0);
            let mut img = RgbaImage::new(w as u32, h as u32);
            for x in 0..(w as u32) {
                for y in 0..(h as u32) {
                    let xy = Vector2::new(x, y);
                    let clip_xy = Vector2::new(
                        2.0 * ((xy.x as f32) / (w as f32)) - 1.0,
                        2.0 * ((xy.y as f32) / (h as f32)) - 1.0,
                    );
                    let rgb = if let Some(pos) = P::clip_to_world_space(&clip_xy) {
                        let pos = pos.truncate().normalize();
                        Rgba([
                            ((pos.x * 0.5 + 0.5) * 256.0) as u8,
                            ((pos.y * 0.5 + 0.5) * 256.0) as u8,
                            ((pos.z * 0.5 + 0.5) * 256.0) as u8,
                            255,
                        ])
                    } else {
                        Rgba([255, 255, 255, 255])
                    };

                    img.put_pixel(x as u32, y as u32, rgb);
                }
            }
            img.save(filename).unwrap();
        }

        generate_projection_map::<Aitoff>("./img/aitoff2.png");
        /*generate_projection_map::<Gnomonic>("./img/tan.png");
        generate_projection_map::<AzimuthalEquidistant>("./img/arc.png");
        generate_projection_map::<Mollweide>("./img/mollweide.png");
        generate_projection_map::<Mercator>("./img/mercator.png");
        generate_projection_map::<Orthographic>("./img/sinus.png");*/
    }
}
