pub struct Bounds<T> {
  lower: T,
  upper: T
}

pub trait Bounded<const N: usize> where Self: PartialOrd {
  // Get the index of which bound the value falls in
  fn which_bounds(&self, bounds: [Bounds<Self>; N]) -> Option<usize>;
}

impl Bounded<f32, const N: usize> for f32 {
  fn which_bounds(&self, bounds: [Bounds<f32>; N]) -> Option<usize> {
    for (index, bound) in bounds.iter().enumerate() {
      if bound.lower <= self && self <= bound.upper {
        Some(index)
      }
    }
    None
  }
}



