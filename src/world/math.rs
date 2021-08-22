use bevy::prelude::*;

pub fn is_within_cubic_bounds(pos: IVec3, min: i32, max: i32) -> bool {
    pos.min_element() >= min && pos.max_element() <= max
}

pub fn floor(vec: Vec3) -> IVec3 {
    IVec3::new(
        vec.x.floor() as i32,
        vec.y.floor() as i32,
        vec.z.floor() as i32,
    )
}

pub fn euclid_rem(vec: IVec3, div: i32) -> IVec3 {
    IVec3::new(
        vec.x.rem_euclid(div),
        vec.y.rem_euclid(div),
        vec.z.rem_euclid(div),
    )
}

pub enum Vec3Element {
    X,
    Y,
    Z,
}

pub fn min_element(vec: Vec3) -> Vec3Element {
    let vec = vec.abs();

    if vec.x < vec.y && vec.x < vec.z {
        Vec3Element::X
    } else if vec.y < vec.x && vec.y < vec.z {
        Vec3Element::Y
    } else {
        Vec3Element::Z
    }
}

// pub fn get_min_abs_axis(vec: Vec3) -> f32 {
//     let abs = vec.abs();
//     if abs.x < abs.y && abs.x < abs.z {
//         vec.x
//     } else if abs.y < abs.x && abs.y < abs.z {
//         vec.y
//     } else {
//         vec.z
//     }
// }

// pub fn to_unit_axis_ivec3(vec: Vec3) -> IVec3 {
//     let abs = vec.normalize().abs();
//     if abs.x > abs.y && abs.x > abs.z {
//         (vec.x.signum() as i32) * IVec3::X
//     } else if abs.y > abs.x && abs.y > abs.z {
//         (vec.y.signum() as i32) * IVec3::Y
//     } else {
//         (vec.z.signum() as i32) * IVec3::Z
//     }
// }
