pub trait Matcher<T> {
    fn matches(&self, target: &T) -> Option<String>;
}
