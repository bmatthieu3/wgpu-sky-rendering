use crate::angle::Angle;

use cgmath::BaseFloat;
use cgmath::{Vector3, Vector4};

#[allow(dead_code)]
pub type Vec2<T> = cgmath::Vector2<T>;
#[allow(dead_code)]
pub type Vec3<T> = cgmath::Vector3<T>;
#[allow(dead_code)]
pub type Vec4<T> = cgmath::Vector4<T>;

#[allow(dead_code)]
pub type Mat4<T> = cgmath::Matrix4<T>;
#[allow(dead_code)]
pub type Mat3<T> = cgmath::Matrix3<T>;

use cgmath::num_traits::FloatConst;
pub trait Float: cgmath::BaseFloat + FloatConst {}
impl Float for f32 {}
impl Float for f64 {}

#[allow(dead_code)]
#[inline]
pub fn xyz_to_radec<S: BaseFloat>(v: &cgmath::Vector3<S>) -> (Angle<S>, Angle<S>) {
    let lon = Angle(v.x.atan2(v.z));
    let lat = Angle(v.y.atan2((v.x * v.x + v.z * v.z).sqrt()));

    (lon, lat)
}
#[allow(dead_code)]
#[inline]
pub fn xyzw_to_radec<S: BaseFloat>(v: &cgmath::Vector4<S>) -> (Angle<S>, Angle<S>) {
    let lon = Angle(v.x.atan2(v.z));
    let lat = Angle(v.y.atan2((v.x * v.x + v.z * v.z).sqrt()));

    (lon, lat)
}

#[inline]
#[allow(dead_code)]
pub fn radec_to_xyzw<S: BaseFloat>(theta: Angle<S>, delta: Angle<S>) -> Vector4<S> {
    Vector4::<S>::new(
        delta.cos() * theta.sin(),
        delta.sin(),
        delta.cos() * theta.cos(),
        S::one(),
    )
}

#[inline]
#[allow(dead_code)]
pub fn radec_to_xyz<S: BaseFloat>(theta: Angle<S>, delta: Angle<S>) -> Vector3<S> {
    Vector3::<S>::new(
        delta.cos() * theta.sin(),
        delta.sin(),
        delta.cos() * theta.cos(),
    )
}

#[inline]
pub fn asinc_positive<T: Float>(mut x: T) -> T {
    assert!(x >= T::zero());
    if x > T::from(1.0e-4).unwrap() {
        x.asin() / x
    } else {
        // If a is mall, use Taylor expension of asin(a) / a
        // a = 1e-4 => a^4 = 1.e-16
        x *= x;
        let nine = T::from(9.0).unwrap();
        let twenty = T::from(20.0).unwrap();
        let six = T::from(6.0).unwrap();

        T::one() + x * (T::one() + x * nine / twenty) / six
    }
}

#[inline]
pub fn sinc_positive<T: Float>(mut x: T) -> T {
    assert!(x >= T::zero());
    if x > T::from(1.0e-4).unwrap() {
        x.sin() / x
    } else {
        // If a is mall, use Taylor expension of asin(a) / a
        // a = 1e-4 => a^4 = 1.e-16
        x *= x;
        let _nine = T::from(9.0).unwrap();
        let twenty = T::from(20.0).unwrap();
        let six = T::from(6.0).unwrap();
        T::one() - x * (T::one() - x / twenty) / six
    }
}

use cgmath::InnerSpace;
pub fn rotation_from_direction<T: Float>(direction: &Vec3<T>, up: &Vec3<T>) -> Mat4<T> {
    let xaxis = up.cross(*direction).normalize();
    let yaxis = direction.cross(xaxis).normalize();

    Mat4::new(
        xaxis.x,
        yaxis.x,
        direction.x,
        T::zero(),

        xaxis.y,
        yaxis.y,
        direction.y,
        T::zero(),

        xaxis.z,
        yaxis.z,
        direction.z,
        T::zero(),

        T::zero(),
        T::zero(),
        T::zero(),
        T::one(),
    )
}
