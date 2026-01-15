use std::collections::HashMap;

pub struct GroupResolver<'a> {
    groups: &'a HashMap<String, Vec<String>>,
    process_names: Vec<String>,
}

impl<'a> GroupResolver<'a> {
    pub fn new(groups: &'a HashMap<String, Vec<String>>, process_names: Vec<String>) -> Self {
        Self {
            groups,
            process_names,
        }
    }

    /// Resolve a name to a list of process names.
    /// - If name matches a group, returns the group members
    /// - If name is "all", returns all process names
    /// - Otherwise returns the name as a single-element list
    pub fn resolve(&self, name: &str) -> Vec<String> {
        if let Some(members) = self.groups.get(name) {
            members.clone()
        } else if name == "all" {
            self.process_names.clone()
        } else {
            vec![name.to_string()]
        }
    }

    /// Check if a name is a group name
    pub fn is_group(&self, name: &str) -> bool {
        self.groups.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_groups() -> HashMap<String, Vec<String>> {
        let mut groups = HashMap::new();
        groups.insert(
            "rails".to_string(),
            vec!["puma".to_string(), "workers".to_string()],
        );
        groups.insert("frontend".to_string(), vec!["webpack".to_string()]);
        groups
    }

    fn test_process_names() -> Vec<String> {
        vec![
            "puma".to_string(),
            "workers".to_string(),
            "webpack".to_string(),
            "redis".to_string(),
        ]
    }

    #[test]
    fn resolve_returns_group_members_for_group_name() {
        let groups = test_groups();
        let resolver = GroupResolver::new(&groups, test_process_names());

        let result = resolver.resolve("rails");
        assert_eq!(result, vec!["puma", "workers"]);
    }

    #[test]
    fn resolve_returns_all_processes_for_all() {
        let groups = test_groups();
        let resolver = GroupResolver::new(&groups, test_process_names());

        let result = resolver.resolve("all");
        assert_eq!(result, vec!["puma", "workers", "webpack", "redis"]);
    }

    #[test]
    fn resolve_returns_single_element_for_process_name() {
        let groups = test_groups();
        let resolver = GroupResolver::new(&groups, test_process_names());

        let result = resolver.resolve("puma");
        assert_eq!(result, vec!["puma"]);
    }

    #[test]
    fn resolve_returns_single_element_for_unknown_name() {
        let groups = test_groups();
        let resolver = GroupResolver::new(&groups, test_process_names());

        let result = resolver.resolve("unknown");
        assert_eq!(result, vec!["unknown"]);
    }

    #[test]
    fn is_group_returns_true_for_group() {
        let groups = test_groups();
        let resolver = GroupResolver::new(&groups, test_process_names());

        assert!(resolver.is_group("rails"));
        assert!(resolver.is_group("frontend"));
    }

    #[test]
    fn is_group_returns_false_for_non_group() {
        let groups = test_groups();
        let resolver = GroupResolver::new(&groups, test_process_names());

        assert!(!resolver.is_group("puma"));
        assert!(!resolver.is_group("all"));
        assert!(!resolver.is_group("unknown"));
    }
}
