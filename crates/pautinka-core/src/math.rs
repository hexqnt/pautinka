use emath::Vec2;

pub(crate) fn unit(vector: Vec2) -> Vec2 {
    let length = vector.length();
    if length > 0.0 {
        vector / length
    } else {
        Vec2::ZERO
    }
}
