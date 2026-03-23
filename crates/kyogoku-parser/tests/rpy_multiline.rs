use kyogoku_parser::ParserRegistry;

#[test]
fn test_rpy_multiline_parsing() {
    let content = r#"
define e = Character("Eileen")

label start:
    e """
    First line of multiline.
    Second line.
    """

    "Just narration multiline
    continues here."

    python:
        x = """
        Should not be parsed as dialogue
        """
"#;

    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.rpy")).unwrap();
    let blocks = parser.parse(content.as_bytes()).unwrap();

    // We expect:
    // 1. Dialogue block "First line...\nSecond line." (speaker "e")
    // 2. Narration block "Just narration multiline..." -> NOT VALID multiline without triple quotes. Will be skipped.
    // 3. Python block should be skipped due to indentation logic.

    assert_eq!(blocks.len(), 1, "Should parse exactly 1 multiline block");
    
    let b0 = &blocks[0];
    assert_eq!(b0.speaker.as_deref(), Some("e"));
    assert!(b0.source.contains("First line"));
    assert!(b0.source.contains("Second line"));
    
    // Ensure python content is NOT present in any block
    for b in blocks {
        assert!(!b.source.contains("Should not be parsed"), "Python block content leaked into translation");
    }
}
