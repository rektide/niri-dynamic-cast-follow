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
    T::Id: Eq + Hash,
    T: Clone,
{
    pub fn new() -> Self {
        TargetState {
            targets: HashMap::new(),
            current_focused_id: None,
        }
    }
}

pub trait FollowerState {
    type Target: Target;
    type Id: Eq + Hash;

    fn get_current_focused_id(&self) -> Option<Self::Id>;
    fn set_current_focused_id(&mut self, id: Option<Self::Id>);
    fn get_targets(&self) -> &HashMap<Self::Id, Self::Target>;
    fn get_targets_mut(&mut self) -> &mut HashMap<Self::Id, Self::Target>;
}

impl<T: Target> FollowerState for TargetState<T>
where
    T::Id: Eq + Hash + Clone,
{
    type Target = T;
    type Id = T::Id;

    fn get_current_focused_id(&self) -> Option<Self::Id> {
        self.current_focused_id.clone()
    }

    fn set_current_focused_id(&mut self, id: Option<Self::Id>) {
        self.current_focused_id = id;
    }

    fn get_targets(&self) -> &HashMap<Self::Id, Self::Target> {
        &self.targets
    }

    fn get_targets_mut(&mut self) -> &mut HashMap<Self::Id, Self::Target> {
        &mut self.targets
    }
}
