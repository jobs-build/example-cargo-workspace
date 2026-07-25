/// The one thing this workspace ships.
pub fn greeting() -> String {
    "hello from crates/greeting".to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn greets() {
        assert!(super::greeting().contains("hello"));
    }
}
