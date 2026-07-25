use heck::ToSnakeCase;

/// Deliberately NOT in app's dependency closure: this crate (and its external
/// dependency, `heck`) exist to prove that the cargo plugin's pruned lockfile
/// and reduced workspace manifest keep them out of app's build identity.
pub fn snake(s: &str) -> String {
    s.to_snake_case()
}
