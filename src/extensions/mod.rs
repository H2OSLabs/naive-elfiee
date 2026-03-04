/// Extensions for Elfiee capability system.
///
/// Extensions provide domain-specific functionality for different block types.
///
/// ## Available Extensions
///
/// - `document`: Read and write content to document blocks (unified markdown + code)
/// - `task`: Task management with commit auditing
/// - `session`: Append-only session recording (commands, messages, decisions)
pub mod document;
pub mod session;
pub mod task;
