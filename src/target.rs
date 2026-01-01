use std::collections::HashMap;
use std::hash::Hash;

pub trait Target {
    type Id: Eq + Hash;
    fn id(&self) -> Self::Id;
}

pub struct TargetState<T: Target>
where
    T::Id: Eq + Hash,
{
    pub targets: HashMap<T::Id, T>,
    pub current_focused_id: Option<T::Id>,
}

impl<T: Target> TargetState<T>
where
    T::Id: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        TargetState {
            targets: HashMap::new(),
            current_focused_id: None,
        }
    }
}
