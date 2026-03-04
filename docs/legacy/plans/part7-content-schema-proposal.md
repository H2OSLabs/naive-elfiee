# Part 7: Content Schema Design Proposal

> **Status**: Proposal for future implementation
> **Priority**: Low (post-MVP)
> **Context**: Currently, block content schemas are untyped (`serde_json::Value`). This proposal suggests adding optional typed content support while maintaining flexibility.

# Content Schema Design Proposal

## Problem Statement

Currently, block types have no formal content schema definition:
- `Block.contents` is `serde_json::Value` (untyped JSON)
- Capabilities validate payload at runtime
- No compile-time guarantees about content structure
- Difficult to document and maintain

## Design Goals

1. **Type Safety**: Catch schema errors at compile time when possible
2. **Flexibility**: Support schema evolution for event-sourced data
3. **Backward Compatibility**: Don't break existing untyped extensions
4. **Developer Experience**: Clear documentation and IDE support

## Proposed Solution: Typed Content with Schema Registry

### 1. Define Content Types

Each block type can optionally define its content structure:

```rust
// src/extensions/markdown/content.rs
use serde::{Deserialize, Serialize};

/// Content schema for markdown blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownContent {
    /// The markdown text content
    pub markdown: String,

    /// Optional metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MarkdownMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownMetadata {
    /// Word count
    pub word_count: Option<usize>,

    /// Last modified timestamp
    pub last_modified: Option<String>,
}

impl Default for MarkdownContent {
    fn default() -> Self {
        Self {
            markdown: String::new(),
            metadata: None,
        }
    }
}
```

### 2. Typed Block Accessor

Add helper methods to work with typed content:

```rust
// src/models/block.rs
impl Block {
    /// Try to deserialize contents as a specific type.
    pub fn contents_as<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.contents.clone())
    }

    /// Try to deserialize contents, returning default if empty/invalid.
    pub fn contents_as_or_default<T>(&self) -> T
    where
        T: serde::de::DeserializeOwned + Default
    {
        self.contents_as::<T>().unwrap_or_default()
    }

    /// Set contents from a typed value.
    pub fn set_contents<T: serde::Serialize>(&mut self, value: &T) -> Result<(), serde_json::Error> {
        self.contents = serde_json::to_value(value)?;
        Ok(())
    }
}
```

### 3. Update Capabilities to Use Typed Content

**Before (untyped):**
```rust
#[capability(id = "markdown.write", target = "markdown")]
fn handle_markdown_write(cmd: &Command, block: &Block) -> CapResult<Vec<Event>> {
    let markdown_content = cmd.payload.get("content")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'content' in payload")?;

    let mut new_contents = if let Some(obj) = block.contents.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };
    new_contents.insert("markdown".to_string(), serde_json::json!(markdown_content));

    let event = create_event(
        block.block_id.clone(),
        "markdown.write",
        serde_json::json!({ "contents": new_contents }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

**After (typed):**
```rust
#[capability(id = "markdown.write", target = "markdown")]
fn handle_markdown_write(cmd: &Command, block: &Block) -> CapResult<Vec<Event>> {
    let new_text = cmd.payload.get("content")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'content' in payload")?;

    // Load current content (with type safety)
    let mut content: MarkdownContent = block.contents_as_or_default();

    // Update typed field
    content.markdown = new_text.to_string();

    // Update metadata
    if let Some(ref mut meta) = content.metadata {
        meta.word_count = Some(new_text.split_whitespace().count());
    }

    let event = create_event(
        block.block_id.clone(),
        "markdown.write",
        serde_json::json!({ "contents": serde_json::to_value(&content)? }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

### 4. Schema Registry (Optional)

For runtime validation and documentation:

```rust
// src/models/schema_registry.rs
use serde_json::Value;
use std::collections::HashMap;

pub type SchemaValidator = Box<dyn Fn(&Value) -> Result<(), String> + Send + Sync>;

/// Registry for block type content schemas.
pub struct SchemaRegistry {
    validators: HashMap<String, SchemaValidator>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            validators: HashMap::new(),
        };
        registry.register_builtin_schemas();
        registry
    }

    /// Register a schema validator for a block type.
    pub fn register(&mut self, block_type: String, validator: SchemaValidator) {
        self.validators.insert(block_type, validator);
    }

    /// Validate contents against registered schema.
    pub fn validate(&self, block_type: &str, contents: &Value) -> Result<(), String> {
        if let Some(validator) = self.validators.get(block_type) {
            validator(contents)
        } else {
            // No schema registered - allow any content
            Ok(())
        }
    }

    fn register_builtin_schemas(&mut self) {
        // Core block type (minimal schema)
        self.register(
            "core".to_string(),
            Box::new(|_value| Ok(())),
        );
    }
}
```

### 5. Register Schema in Extension

```rust
// src/extensions/markdown/mod.rs
pub fn register_markdown_schema(registry: &mut SchemaRegistry) {
    registry.register(
        "markdown".to_string(),
        Box::new(|value| {
            // Try to deserialize as MarkdownContent
            serde_json::from_value::<MarkdownContent>(value.clone())
                .map(|_| ())
                .map_err(|e| format!("Invalid markdown content: {}", e))
        }),
    );
}
```

## Migration Strategy

### Phase 1: Add Typed Accessors (Non-Breaking)
- Add `contents_as()` and `set_contents()` to Block
- Existing code continues to work
- New code can opt into type safety

### Phase 2: Add Content Type Definitions (Optional)
- Extensions can define content types
- Use typed accessors in capabilities
- Untyped extensions still work

### Phase 3: Add Schema Registry (Optional)
- Runtime validation for debugging
- Schema documentation generation
- Migration tools for old data

## Benefits

### For Type Safety
```rust
// Compile-time error if field name is wrong
let content: MarkdownContent = block.contents_as()?;
let text = content.markdown; // IDE autocomplete!

// vs untyped
let text = block.contents.get("markdwon") // typo not caught!
    .and_then(|v| v.as_str());
```

### For Documentation
```rust
/// Content schema for markdown blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownContent {
    /// The markdown text content
    pub markdown: String,
    // ^ This is self-documenting!
}
```

### For Schema Evolution
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownContent {
    pub markdown: String,

    /// New field in v2, optional for backward compatibility
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v2_feature: Option<String>,
}
```

### For Validation
```rust
// Validate when creating blocks
let content = MarkdownContent::default();
block.set_contents(&content)?; // Type-checked at compile time

// Or at runtime
schema_registry.validate("markdown", &block.contents)?;
```

## Drawbacks

1. **More Boilerplate**: Need to define structs for each block type
2. **Schema Evolution Complexity**: Old events need careful handling
3. **Performance**: Serialization/deserialization overhead
4. **Mixed Approach**: Both typed and untyped code in codebase

## Recommendation

**Adopt a hybrid approach:**

1. **Keep `Block.contents` as `Value`** for flexibility
2. **Add typed accessors** (`contents_as()`, `set_contents()`)
3. **Recommend** (but don't require) content type definitions for extensions
4. **Provide examples** of both typed and untyped approaches in docs

This gives developers:
- **Choice**: Use types when beneficial, skip when not needed
- **Safety**: Opt into type safety without breaking flexibility
- **Evolution**: Handle old data gracefully
- **Documentation**: Types serve as living documentation

## Example Implementation

See:
- `src/models/block.rs` - Add typed accessors
- `src/extensions/markdown/content.rs` - Example content type
- `src/extensions/markdown/markdown_write.rs` - Example typed capability
- `docs/guides/EXTENSION_DEVELOPMENT.md` - Update with schema patterns

## Open Questions

1. Should we enforce schema validation at block creation time?
2. Should schemas be versioned explicitly (e.g., `MarkdownContentV1`, `V2`)?
3. Should we auto-generate schema documentation from types?
4. Should we provide migration utilities for schema evolution?
5. How do we handle polymorphic content (blocks that can have multiple schemas)?

## Next Steps

If this proposal is accepted:
1. Implement Phase 1 (typed accessors)
2. Refactor Markdown extension to use typed content
3. Update extension development guide
4. Add schema examples to documentation
5. Consider Phase 2/3 based on developer feedback
