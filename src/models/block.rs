use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 唯一允许的 relation type，表示"上游定义/决定下游"的因果关系。
///
/// `Block.children` 的 key 仅允许使用此常量值。
/// 语义：`A.children["implement"] = [B]` 表示 A 的改动导致 B 需要改动。
/// 例如：Task → Document, PRD → Task → Test
pub const RELATION_IMPLEMENT: &str = "implement";

/// Block 是 Elfiee 的基本内容单元。
///
/// `children` 字段存储逻辑因果关系图（Logical Causal Graph），
/// key 仅允许 `RELATION_IMPLEMENT`（即 `"implement"`），
/// value 为下游 block_id 列表。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,

    /// Block 描述（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Block {
    pub fn new(name: String, block_type: String, owner: String) -> Self {
        Self {
            block_id: uuid::Uuid::new_v4().to_string(),
            name,
            block_type,
            contents: serde_json::json!({}),
            children: HashMap::new(),
            owner,
            description: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_block_has_no_description() {
        let block = Block::new(
            "Test Block".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        assert_eq!(block.name, "Test Block");
        assert_eq!(block.block_type, "document");
        assert_eq!(block.owner, "alice");
        assert!(block.description.is_none());
    }

    #[test]
    fn test_block_with_description() {
        let mut block = Block::new(
            "Test".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        block.description = Some("测试描述".to_string());
        assert_eq!(block.description, Some("测试描述".to_string()));
    }
}
