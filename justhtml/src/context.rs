//! Fragment parsing context.
//!
//! Corresponds to Python's `context.py`.

/// Context for fragment parsing, specifying the context element.
#[derive(Debug, Clone)]
pub struct FragmentContext {
    pub tag_name: String,
    pub namespace: Option<String>,
}

impl FragmentContext {
    pub fn new(tag_name: &str, namespace: Option<&str>) -> Self {
        Self {
            tag_name: tag_name.to_string(),
            namespace: namespace.map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragment_context_html() {
        let ctx = FragmentContext::new("div", None);
        assert_eq!(ctx.tag_name, "div");
        assert!(ctx.namespace.is_none());
    }

    #[test]
    fn test_fragment_context_with_namespace() {
        let ctx = FragmentContext::new("svg", Some("http://www.w3.org/2000/svg"));
        assert_eq!(ctx.tag_name, "svg");
        assert_eq!(ctx.namespace.as_deref(), Some("http://www.w3.org/2000/svg"));
    }
}
